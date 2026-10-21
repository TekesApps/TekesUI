//! 字体创建与字体身份登记。
//!
//! CoreText 的字体（含排版时自动回退出来的字体）通过 [`CoreTextFonts`] 映射成
//! [`FontKey`]。身份键参照参照实现的做法：PostScript 名、字号、符号特征、字体矩阵、
//! 变体轴取值；同一身份得到同一个 `FontKey`，登记顺序决定编号，所以同一进程里同一
//! 登记表的结果是确定的。
//!
//! 登记表不是 `Send`（CTFont 在 objc2 里没有标 `Send`/`Sync`），排版器和光栅化器要
//! 在同一线程上共享同一个 [`CoreTextFonts`]（克隆是共享，不是复制）。

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::rc::Rc;

use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_text::{
    CTFont, CTFontDescriptor, CTFontSymbolicTraits, CTFontUIFontType, kCTFontTraitsAttribute,
    kCTFontWeightTrait,
};

use crate::model::FontInfo;
use crate::style::{FontFamily, TextStyle};
use tekes_ui_scene::FontKey;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Identity {
    post_script_name: String,
    size_bits: u64,
    traits: u32,
    matrix: [u64; 6],
    /// (轴标识, 取值的 f64 位模式)，按轴标识排序。
    variation: Vec<(i64, u64)>,
}

struct Entry {
    font: CFRetained<CTFont>,
    info: FontInfo,
}

#[derive(Default)]
struct Inner {
    by_identity: HashMap<Identity, FontKey>,
    entries: Vec<Entry>,
}

/// CoreText 字体登记表。克隆得到同一张表的另一个句柄。
#[derive(Clone, Default)]
pub struct CoreTextFonts {
    inner: Rc<RefCell<Inner>>,
}

impl std::fmt::Debug for CoreTextFonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreTextFonts")
            .field("fonts", &self.inner.borrow().entries.len())
            .finish()
    }
}

/// CSS 风格的 100..=900 字重映射到 CoreText 的 `kCTFontWeightTrait`（-1..1）。
/// 数值是 AppKit 的 `NSFontWeight*` 常量。
fn weight_trait(weight: u16) -> f64 {
    match weight {
        0..=100 => -0.8,
        101..=200 => -0.6,
        201..=300 => -0.4,
        301..=400 => 0.0,
        401..=500 => 0.23,
        501..=600 => 0.3,
        601..=700 => 0.4,
        701..=800 => 0.56,
        _ => 0.62,
    }
}

impl CoreTextFonts {
    pub fn new() -> CoreTextFonts {
        CoreTextFonts::default()
    }

    /// 按样式创建 CTFont（不登记）。
    ///
    /// 系统界面字体用 `CTFontCreateUIFontForLanguage(kCTFontUIFontSystem, size, nil)`，
    /// 失败时退回 `.AppleSystemUIFont`。字重不是 400 时，用字重特征生成变体。
    pub fn create_font(&self, style: &TextStyle) -> CFRetained<CTFont> {
        let size = f64::from(style.size);
        let base = match &style.family {
            FontFamily::SystemUi => {
                // SAFETY: 参数都是有效值，language 传 None。
                unsafe { CTFont::new_ui_font_for_language(CTFontUIFontType::System, size, None) }
                    .unwrap_or_else(|| {
                        let name = CFString::from_str(".AppleSystemUIFont");
                        // SAFETY: matrix 传空指针表示单位矩阵。
                        unsafe { CTFont::with_name(&name, size, ptr::null()) }
                    })
            }
            FontFamily::Named(name) => {
                let name = CFString::from_str(name);
                // SAFETY: 同上。
                unsafe { CTFont::with_name(&name, size, ptr::null()) }
            }
        };
        if style.weight == 400 {
            return base;
        }
        let weight = CFNumber::new_f64(weight_trait(style.weight));
        let traits = CFDictionary::<CFString, CFType>::from_slices(
            &[unsafe { kCTFontWeightTrait }],
            &[weight.as_ref()],
        );
        let attrs = CFDictionary::<CFString, CFType>::from_slices(
            &[unsafe { kCTFontTraitsAttribute }],
            &[traits.as_ref()],
        );
        // SAFETY: attrs 是 CFString 到 CFType 的字典，符合描述符属性的要求。
        let descriptor = unsafe { CTFontDescriptor::with_attributes(attrs.as_opaque()) };
        // SAFETY: matrix 为空指针，size 保持原值，descriptor 有效。
        unsafe { base.copy_with_attributes(0.0, ptr::null(), Some(&descriptor)) }
    }

    /// 登记一个字体并返回它的 `FontKey`；已登记的身份返回原来的键。
    pub fn key_for(&self, font: &CTFont) -> FontKey {
        // Registered fonts are retained by this registry: pointer identity is
        // safe for the common repeated CTRun font, without rebuilding variation
        // dictionaries and PostScript strings for every wrapped line.
        if let Some(key) = self
            .inner
            .borrow()
            .entries
            .iter()
            .find_map(|entry| ptr::eq(entry.font.as_ref(), font).then_some(entry.info.key))
        {
            return key;
        }
        let identity = identity_of(font);
        let mut inner = self.inner.borrow_mut();
        if let Some(key) = inner.by_identity.get(&identity) {
            return *key;
        }
        let key = FontKey(inner.entries.len() as u32);
        // SAFETY: font 是有效的 CTFont；这里只读属性。
        let (size, traits) = unsafe { (font.size(), font.symbolic_traits()) };
        let info = FontInfo {
            key,
            post_script_name: identity.post_script_name.clone(),
            size,
            is_color: traits.contains(CTFontSymbolicTraits::TraitColorGlyphs),
        };
        // 保留一份字体的强引用，光栅化时按键取回。
        let retained = retain_font(font);
        inner.entries.push(Entry {
            font: retained,
            info,
        });
        inner.by_identity.insert(identity, key);
        key
    }

    pub fn info(&self, key: FontKey) -> Option<FontInfo> {
        self.inner
            .borrow()
            .entries
            .get(key.0 as usize)
            .map(|e| e.info.clone())
    }

    pub fn font(&self, key: FontKey) -> Option<CFRetained<CTFont>> {
        self.inner
            .borrow()
            .entries
            .get(key.0 as usize)
            .map(|e| e.font.clone())
    }

    /// 已登记的字体数。
    pub fn len(&self) -> usize {
        self.inner.borrow().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn retain_font(font: &CTFont) -> CFRetained<CTFont> {
    // SAFETY: font 是有效引用；CFRetained::retain 对它加一次引用计数。
    unsafe { CFRetained::retain(ptr::NonNull::from(font)) }
}

fn identity_of(font: &CTFont) -> Identity {
    // SAFETY: 全是只读属性查询。
    unsafe {
        let post_script_name = font.post_script_name().to_string();
        let size_bits = font.size().to_bits();
        let traits = font.symbolic_traits().0;
        let m = font.matrix();
        let matrix = [m.a, m.b, m.c, m.d, m.tx, m.ty].map(f64::to_bits);
        let mut variation = Vec::new();
        if let Some(dict) = font.variation() {
            let n = dict.count() as usize;
            let mut keys: Vec<*const c_void> = vec![ptr::null(); n];
            let mut values: Vec<*const c_void> = vec![ptr::null(); n];
            dict.keys_and_values(keys.as_mut_ptr(), values.as_mut_ptr());
            for (k, v) in keys.into_iter().zip(values) {
                // 变体字典的键和值都是 CFNumber。
                let k = &*(k as *const CFNumber);
                let v = &*(v as *const CFNumber);
                if let (Some(k), Some(v)) = (k.as_i64(), v.as_f64()) {
                    variation.push((k, v.to_bits()));
                }
            }
            variation.sort_unstable();
        }
        Identity {
            post_script_name,
            size_bits,
            traits,
            matrix,
            variation,
        }
    }
}
