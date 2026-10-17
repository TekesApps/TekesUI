//! Optional public MTLDrawable presented callbacks. No method swizzling or SPI.
use block2::RcBlock;
use objc2::{
    AllocAnyThread, DefinedClass, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_foundation::NSObjectProtocol;
use objc2_metal::MTLDrawable;
use objc2_quartz_core::{CACurrentMediaTime, CAMetalDrawable, CAMetalLayer};
use std::{
    cell::RefCell,
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
#[derive(Clone, Copy, Debug)]
pub struct PresentedFrame {
    pub serial: u64,
    pub acquired_at: f64,
    pub presented_at: f64,
}
#[derive(Debug, Default)]
pub(crate) struct PresentationObserver {
    pub enabled: AtomicBool,
    pub metal_display_link: AtomicBool,
    pub serial: AtomicU64,
    pub provided_drawables: AtomicU64,
    pub fallback_acquires: AtomicU64,
    pub frames: Mutex<Vec<PresentedFrame>>,
}
type Drawable = Retained<ProtocolObject<dyn CAMetalDrawable>>;
thread_local! {
    // Main-thread callback scope only. Metal objects never cross a Rust thread.
    static PROVIDED: RefCell<Vec<(usize, Option<Drawable>)>> = const { RefCell::new(Vec::new()) };
}

/// Hand the display link's drawable to the synchronous render callback. The HAL
/// still uses its ordinary surface path; our owned layer returns this exact
/// retained drawable once, without acquiring another from the compositor.
pub(crate) fn with_provided_drawable<R>(
    layer: &CAMetalLayer,
    drawable: Drawable,
    f: impl FnOnce() -> R,
) -> R {
    let key = layer as *const CAMetalLayer as usize;
    PROVIDED.with(|slot| slot.borrow_mut().push((key, Some(drawable))));
    struct Scope;
    impl Drop for Scope {
        fn drop(&mut self) {
            PROVIDED.with(|slot| {
                slot.borrow_mut().pop();
            });
        }
    }
    let _scope = Scope;
    f()
}

// None: ordinary layer acquisition. Some(None): this callback's supplied
// drawable was consumed already; a surface retry must wait for another update.
fn take_provided(layer: &CAMetalLayer) -> Option<Option<Drawable>> {
    let key = layer as *const CAMetalLayer as usize;
    PROVIDED.with(|slot| {
        slot.borrow_mut()
            .iter_mut()
            .rev()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, drawable)| drawable.take())
    })
}

// SAFETY: CAMetalLayer has no additional subclass invariants. The observer uses
// only Arc/atomics/Mutex because Metal invokes presented handlers off-main-thread.
define_class!(
#[unsafe(super=CAMetalLayer)]
#[ivars=Arc<PresentationObserver>]
#[derive(Debug)]
pub(crate) struct ObservedMetalLayer;
unsafe impl NSObjectProtocol for ObservedMetalLayer {}
impl ObservedMetalLayer {
 #[unsafe(method_id(initWithLayer:))]
 fn init_with_layer(this:objc2::rc::Allocated<Self>,other:&AnyObject)->Retained<Self> {
  let observer=other.downcast_ref::<Self>().map(|layer|layer.ivars().clone()).unwrap_or_default();
  let this=this.set_ivars(observer);
  // SAFETY: exact CALayer copy initializer; Rust state is copied before super.
  unsafe{msg_send![super(this),initWithLayer:other]}
 }
 #[unsafe(method_id(nextDrawable))]
 fn next_drawable(&self)->Option<Retained<ProtocolObject<dyn CAMetalDrawable>>> {
  // SAFETY: exact CAMetalLayer nextDrawable ABI and ordinary retained result.
  let provided=take_provided(self);
  let is_provided=provided.is_some();
  if self.ivars().enabled.load(Ordering::Relaxed) {
   if provided.as_ref().is_some_and(|v|v.is_some()) { self.ivars().provided_drawables.fetch_add(1,Ordering::Relaxed); }
   else if provided.is_none() { self.ivars().fallback_acquires.fetch_add(1,Ordering::Relaxed); }
  }
  let drawable:Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>=match provided {
   Some(drawable)=>drawable,
   None=>unsafe{msg_send![super(self),nextDrawable]},
  };
  // CAMetalDisplayLink may acquire upcoming drawables internally. Observe only
  // the one handed to our renderer, so serials refer to submitted frames.
  let observe=is_provided || !self.ivars().metal_display_link.load(Ordering::Relaxed);
  if let Some(drawable)=&drawable && observe && self.ivars().enabled.load(Ordering::Relaxed) {
   let observer=self.ivars().clone();
   let serial=observer.serial.fetch_add(1,Ordering::Relaxed)+1;
   let acquired_at=CACurrentMediaTime();
   let callback=RcBlock::new(move |d:NonNull<ProtocolObject<dyn MTLDrawable>>|{
    // SAFETY: Metal guarantees a live drawable for the duration of its callback.
    let presented_at=unsafe{d.as_ref()}.presentedTime();
    if let Ok(mut frames)=observer.frames.lock(){frames.push(PresentedFrame{serial,acquired_at,presented_at});}
   });
   // SAFETY: Metal copies the valid block. It owns only an Arc, not the drawable
   // or layer, so completing this callback cannot form a retain cycle.
   unsafe{drawable.addPresentedHandler(RcBlock::as_ptr(&callback));}
  }
  drawable
 }
});
impl ObservedMetalLayer {
    pub fn new(observer: Arc<PresentationObserver>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(observer);
        // SAFETY: CAMetalLayer's inherited initializer; initialized Rust ivars above.
        unsafe { msg_send![super(this), init] }
    }
}
