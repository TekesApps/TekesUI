//! Finite document story viewport. Only explanatory chapters own an outer scroll;
//! the original text surfaces keep their native editing/reading scroll owner.
use super::{Entry, apphost_text_pages};
use crate::gallery::locale::Locale;
use tekes_ui_controls::{composition::*, layout::Insets};
use tekes_ui_tokens::Spacing;
const ROOT: ComponentId = ComponentId(99100);
const COPY: ComponentId = ComponentId(99103);
const HEADER: ComponentId = ComponentId(99106);
const SPECIMEN_WRAP: ComponentId = ComponentId(99107);
const COMMAND: u64 = 99200;
pub(crate) struct AppHostDocuments {
    pub(crate) host: ContentHost,
    entry: &'static Entry,
    locale: Locale,
    section: usize,
    retained: Option<Content>,
    live_id: ComponentId,
    calibration: Option<apphost_text_pages::CalibrationPage>,
}
impl AppHostDocuments {
    pub(crate) fn new(entry: &'static Entry) -> anyhow::Result<Self> {
        let locale = std::env::var("TEKESUI_GALLERY_LOCALE")
            .ok()
            .as_deref()
            .and_then(Locale::from_code)
            .unwrap_or_default();
        let (live, calibration) = Self::live(entry.renderer(), locale);
        let live_id = live.id();
        let inset = Spacing::Loose.value();
        let header = StackContent::vertical(
            Spacing::Regular.value(),
            vec![
                super::story_heading(99101, entry.title()),
                super::apphost_basic::section_bar(entry, COMMAND),
            ],
        )
        .into_content_with_padding(
            HEADER,
            Insets::new(inset, inset, Spacing::Regular.value(), inset),
        );
        let mut host = ContentHost::new(Content::new(
            ROOT,
            StackContent::vertical(0., vec![header, live]).with_flexible_child(live_id),
        ))
        .map_err(|error| anyhow::anyhow!("Gallery document composition: {error:?}"))?;
        host.set_action_events(true);
        Ok(Self {
            host,
            entry,
            locale,
            section: 1,
            retained: None,
            live_id,
            calibration,
        })
    }
    fn live(
        renderer: &str,
        locale: Locale,
    ) -> (Content, Option<apphost_text_pages::CalibrationPage>) {
        let inset = Spacing::Loose.value();
        let content_insets = Insets::new(Spacing::Regular.value(), inset, inset, inset);
        if renderer == "calibration" {
            let page = apphost_text_pages::CalibrationPage::new(locale);
            let live = Content::new(
                ComponentId(99105),
                ScrollContent::new(tekes_ui_controls::ScrollArea::default(), page.content())
                    .content_insets(content_insets)
                    .scrollbar(true),
            );
            (live, Some(page))
        } else {
            let live =
                apphost_text_pages::content(renderer, locale).expect("supported document renderer");
            let id = live.id();
            let wrapped = StackContent::vertical(0., vec![live])
                .with_flexible_child(id)
                .into_content_with_padding(SPECIMEN_WRAP, content_insets);
            (wrapped, None)
        }
    }
    pub(crate) fn prepare_resources(
        &mut self,
        context: &mut tekes_ui_app_host::ContainerRenderContext<'_>,
    ) -> anyhow::Result<()> {
        if self.section == 1
            && let Some(page) = &mut self.calibration
        {
            page.prepare_resources(context)?;
        }
        Ok(())
    }
    fn copy(&self, section: usize) -> Content {
        let mut lines = if section == 0 {
            vec![super::story_docs(self.entry).to_owned(), format!("Component: {}\nUsage: {}", self.entry.component(), super::api_reference(self.entry)), "The original live text owner preserves native selection, clipboard, IME and undo. State is local to this route; Reset restores the original fixture.".into()]
        } else {
            vec![super::behavior_copy(self.entry.renderer()).into()]
        };
        if section == 0 {
            lines.extend(super::contract_summary(self.entry));
        }
        let inset = Spacing::Loose.value();
        Content::new(
            COPY,
            ScrollContent::new(
                tekes_ui_controls::ScrollArea::default(),
                Content::new(
                    ComponentId(99104),
                    ListContent::new(
                        "Document story guidance",
                        lines
                            .into_iter()
                            .enumerate()
                            .map(|(i, line)| {
                                Content::new(ComponentId(99110 + i as u64), TextContent::new(line))
                            })
                            .collect(),
                    ),
                ),
            )
            .content_insets(Insets::new(Spacing::Regular.value(), inset, inset, inset))
            .scrollbar(true),
        )
    }
    pub(crate) fn select_section(&mut self, section: usize) {
        if self.section == section {
            return;
        }
        // Release native focus before removing its original surface subtree.
        self.host.focus(None, false);
        if self.section == 1 {
            self.retained = Some(self.host.remove(self.live_id).expect("live document"));
        } else {
            self.host.remove(COPY).expect("document guidance");
        }
        let content = if section == 1 {
            self.retained.take().expect("retained document")
        } else {
            self.copy(section)
        };
        let id = content.id();
        self.host
            .insert(ROOT, content)
            .expect("unique document chapter");
        self.host
            .get_mut(ROOT)
            .unwrap()
            .component_mut::<StackContent>()
            .unwrap()
            .set_flexible_child(Some(id));
        self.section = section;
    }
    pub(crate) fn after_host_visit(&mut self) {
        for record in self.host.drain_action_records() {
            if record.action.kind != ContentActionKind::Activate {
                continue;
            }
            let command = record.action.target.0.checked_sub(COMMAND);
            if let Some(command @ (1 | 2 | 4)) = command {
                self.select_section(command as usize - 1);
            } else if command == Some(0) {
                let (live, calibration) = Self::live(self.entry.renderer(), self.locale);
                self.calibration = calibration;
                if self.section == 1 {
                    self.host.focus(None, false);
                    self.host.remove(self.live_id).expect("live document");
                    self.host.insert(ROOT, live).expect("reset document");
                } else {
                    self.retained = Some(live);
                }
            } else {
                apphost_text_pages::action(&mut self.host, record.action.target);
            }
        }
    }
    pub(crate) fn clear_calibration_reference_dir(&mut self) {
        if let Some(calibration) = &mut self.calibration {
            calibration.clear_reference_dir();
        }
    }
    pub(crate) fn set_locale(&mut self, locale: Locale) {
        if self.locale == locale {
            return;
        }
        self.locale = locale;
        if let Some(calibration) = &mut self.calibration {
            calibration.set_locale(locale);
            return;
        }
        let (live, calibration) = Self::live(self.entry.renderer(), locale);
        self.calibration = calibration;
        if self.section == 1 {
            self.host.focus(None, false);
            self.host.remove(self.live_id).expect("live document");
            self.host.insert(ROOT, live).expect("locale document");
        } else {
            self.retained = Some(live);
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn editor_scroll_metrics(&self) -> (f32, f32) {
        if let Some(metrics) = apphost_text_pages::surface_scroll_metrics(&self.host) {
            return metrics;
        }
        let fallback_id = if self.section == 1 {
            self.live_id
        } else {
            COPY
        };
        self.host
            .get(fallback_id)
            .and_then(|node| node.component::<ScrollContent>())
            .map_or((0., 0.), |scroll| {
                (
                    scroll.max_offset().y,
                    scroll.scroll().viewport_size().height,
                )
            })
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scroll_editor_to(&mut self, offset: f32) {
        if apphost_text_pages::scroll_surface_to(&mut self.host, offset) {
            return;
        }
        let fallback_id = if self.section == 1 {
            self.live_id
        } else {
            COPY
        };
        if let Some(scroll) = self
            .host
            .get_mut(fallback_id)
            .and_then(|node| node.component_mut::<ScrollContent>())
        {
            let current = scroll.offset().y;
            scroll.scroll_by(0., offset - current);
        }
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scroll_event(
        &mut self,
        delta_y: f32,
        begin: bool,
        end: bool,
        momentum: bool,
        time: f64,
    ) {
        apphost_text_pages::scroll_surface_event(
            &mut self.host,
            delta_y,
            begin,
            end,
            momentum,
            time,
        );
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scroll_reference_state(&self) -> Option<[f32; 2]> {
        apphost_text_pages::scroll_reference_state(&self.host)
    }
    #[cfg(feature = "selftest")]
    pub(crate) fn scroll_reference_fixture(&self) -> Option<serde_json::Value> {
        apphost_text_pages::scroll_reference_fixture(&self.host)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use tekes_ui_scene::{Rect, Scene};
    #[test]
    fn apphost_document_chapters_keep_finite_viewports_and_original_native_drafts() {
        fn render(page: &mut AppHostDocuments, text: &mut TextEngine, frame: Rect) {
            page.host
                .render(
                    &mut Scene::new(),
                    text,
                    frame,
                    frame,
                    tekes_ui_tokens::Theme::LIGHT.into(),
                )
                .unwrap();
            page.after_host_visit();
        }
        let mut text = TextEngine::new();
        for renderer in ["text", "scroll", "internal-text"] {
            let entry = super::super::entries()
                .iter()
                .find(|entry| entry.renderer() == renderer)
                .unwrap();
            let mut page = AppHostDocuments::new(entry).unwrap();
            page.host.set_window_active(true);
            let frame = Rect::new(0., 0., 420., 640.);
            render(&mut page, &mut text, frame);
            let live = page.host.get(apphost_text_pages::SPECIMEN).unwrap().frame();
            assert!(live.size.height > 0. && live.max_y() <= frame.max_y() + 0.01);
            let id = apphost_text_pages::FIRST_SURFACE;
            let before = page.host.text_surface(id).unwrap().text().to_owned();
            assert!(page.host.focus(Some(id), true));
            page.host.event(&ContentEvent::Commit {
                text: "chapter draft".into(),
                replacement: None,
            });
            let draft = page.host.text_surface(id).unwrap().text().to_owned();
            if renderer == "internal-text" {
                assert_ne!(draft, before);
            } else {
                assert_eq!(draft, before);
            }
            render(&mut page, &mut text, frame);
            assert!(
                page.host
                    .accessibility_action(
                        ComponentId(COMMAND + 1),
                        None,
                        AccessibilityAction::Press
                    )
                    .consumed
            );
            page.after_host_visit();
            render(&mut page, &mut text, frame);
            assert!(page.host.text_surface(id).is_none());
            assert!(
                page.host
                    .accessibility_action(
                        ComponentId(COMMAND + 2),
                        None,
                        AccessibilityAction::Press
                    )
                    .consumed
            );
            page.after_host_visit();
            render(&mut page, &mut text, frame);
            assert_eq!(page.host.text_surface(id).unwrap().text(), draft);
            assert!(
                page.host
                    .get(apphost_text_pages::SPECIMEN)
                    .unwrap()
                    .frame()
                    .max_y()
                    <= frame.max_y() + 0.01
            );
        }
    }
}
