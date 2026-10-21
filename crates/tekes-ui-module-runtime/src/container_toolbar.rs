//! Local placement adapter around the original ToolbarControl, not another UI engine.
use crate::{ProcessComponent, component::*};
use anyhow::{Result, ensure};
use std::collections::HashMap;
use tekes_ui_controls::{
    ButtonTextRenderer, ToolbarAction as ControlAction, ToolbarControl,
    composition::{
        Component, ContentEvent, ContentKeyEvent, ContentResponse, ContentTheme, PointerEvent,
    },
};
use tekes_ui_scene::{ImageId, Rect, Scene};
#[derive(Debug, Default)]
pub(crate) struct ContainerToolbar {
    pub control: ToolbarControl,
    pub frame: Rect,
    source: Option<ServerMessage>,
    window: Option<u64>,
    images: HashMap<String, ImageId>,
    dirty: bool,
    pub active: bool,
    ready: bool,
    enabled: bool,
}
impl ContainerToolbar {
    pub fn set_images(&mut self, images: Vec<(String, ImageId)>) {
        self.images = images.into_iter().collect();
        self.dirty = true;
        self.control.cancel();
        self.ready = false;
    }
    pub fn dirty(&mut self) {
        self.dirty = true;
        self.control.cancel();
    }
    pub fn reset(&mut self) {
        self.source = None;
        self.active = false;
        self.ready = false;
        self.control.set_actions(vec![]);
        self.frame = Rect::default();
    }
    pub fn sync(&mut self, process: &ProcessComponent, enabled: bool) -> Result<bool> {
        let source = process.toolbar_registration();
        if !self.dirty
            && self.enabled == enabled
            && self.source.as_ref().map(|s| (&s.session, s.serial))
                == source.as_ref().map(|s| (&s.session, s.serial))
        {
            return Ok(false);
        }
        let mut actions = Vec::new();
        let mut icons = Vec::new();
        if let Some(packet) = &source
            && let Output::Event(ComponentEvent::Toolbar {
                scope: ToolbarScope::View,
                items,
                ..
            }) = &packet.output
        {
            for item in items {
                let label = if item.command.title.is_empty() {
                    item.accessible_name.clone().unwrap_or_default()
                } else {
                    item.command.title.clone()
                };
                actions.push(ControlAction {
                    id: item.command.id,
                    title: label,
                    enabled: enabled && item.command.enabled,
                });
                if let Some(name) = &item.icon {
                    let image = self.images.get(name).copied().ok_or_else(|| {
                        anyhow::anyhow!("toolbar icon capability missing: {name}")
                    })?;
                    icons.push((item.command.id, image));
                }
            }
        }
        self.control.cancel();
        self.control.set_actions(actions);
        self.control
            .set_overflow_icon(self.images.get("tekes-more").copied());
        for (id, image) in icons {
            self.control.set_action_icon(id, Some(image));
        }
        self.active=source.as_ref().is_some_and(|p|matches!(&p.output,Output::Event(ComponentEvent::Toolbar{scope:ToolbarScope::View,items,..}) if !items.is_empty()));
        self.window = process.window_identity();
        self.source = source;
        self.enabled = enabled;
        self.ready = false;
        self.dirty = false;
        Ok(true)
    }
    pub fn arrange(
        &mut self,
        mut text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
    ) -> Rect {
        let height = if self.active {
            self.control
                .measure(&mut text)
                .height
                .min(frame.size.height.max(0.))
        } else {
            0.
        };
        let toolbar = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, height);
        if toolbar != self.frame {
            self.control.cancel();
            self.ready = false;
        }
        self.frame = toolbar;
        self.control.set_viewport(clip);
        Rect::new(
            frame.min_x(),
            frame.min_y() + height,
            frame.size.width,
            (frame.size.height - height).max(0.),
        )
    }
    pub fn paint(
        &mut self,
        scene: &mut Scene,
        mut text: &mut dyn ButtonTextRenderer,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if !self.active {
            return;
        }
        scene.push_clip(clip);
        self.control
            .render_clipped(scene, &mut text, self.frame, theme.palette, Some(clip));
        scene.pop_clip();
        self.ready = self.frame.size.width > 0. && self.frame.size.height > 0.;
    }
    pub fn paint_overlay(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        theme: ContentTheme,
    ) {
        if self.active {
            self.control
                .paint_overlay(scene, text, self.frame, self.frame, theme);
        }
    }
    pub fn pointer(&mut self, pointer: &PointerEvent) -> ContentResponse {
        if self.active && self.ready {
            self.control.pointer_event(pointer)
        } else {
            ContentResponse::default()
        }
    }
    pub fn key(&mut self, key: &ContentKeyEvent) -> ContentResponse {
        if self.active && self.ready {
            self.control.key_event(key)
        } else {
            ContentResponse::default()
        }
    }
    pub fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if self.active && self.ready {
            self.control.event(event)
        } else {
            ContentResponse::default()
        }
    }
    pub fn take_request(&mut self) -> Result<Option<(Session, RequestBody)>> {
        let Some(command) = self.control.take_action() else {
            return Ok(None);
        };
        let packet = self
            .source
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("toolbar source unavailable"))?;
        let Output::Event(ComponentEvent::Toolbar {
            revision,
            scope: ToolbarScope::View,
            ..
        }) = &packet.output
        else {
            anyhow::bail!("local toolbar scope changed")
        };
        ensure!(self.active, "local toolbar closed");
        Ok(Some((
            packet.session.clone(),
            RequestBody::Invoke {
                command,
                registration: *revision,
                window: self
                    .window
                    .ok_or_else(|| anyhow::anyhow!("window binding required"))?,
            },
        )))
    }
}
