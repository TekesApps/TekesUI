//! Controlled bitmap content for the existing Composition tree.
//!
//! Pixel decoding and asynchronous I/O remain host responsibilities. Register
//! decoded premultiplied RGBA bytes with the existing Renderer and provide the
//! resulting process-local lease here.
use super::CurveSubtreeCapability;
use super::*;
use std::sync::Arc;
use tekes_ui_render::gpu::BitmapImageLease;
use tekes_ui_scene::{ImageQuad, TransformedImageQuad};

/// Explicit accessible naming policy for an image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageSemantics {
    Alternative(String),
    Decorative,
}
impl ImageSemantics {
    fn content_semantics(&self) -> ContentSemantics {
        match self {
            Self::Alternative(name) => ContentSemantics::image_alt(name.clone())
                .expect("ImageSemantics validates alternative text at construction"),
            Self::Decorative => ContentSemantics::decorative_image(),
        }
    }
}

/// How the image fits its arranged content bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFit {
    /// Preserve the whole bitmap and leave unused space around it.
    #[default]
    Contain,
    /// Fill the content bounds and crop the source bitmap.
    Cover,
}

/// Normalized position used to place a contained image or choose its cover crop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageAlignment {
    pub x: f32,
    pub y: f32,
}
impl ImageAlignment {
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };
    pub const TOP_LEADING: Self = Self { x: 0.0, y: 0.0 };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    fn valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && (0.0..=1.0).contains(&self.x)
            && (0.0..=1.0).contains(&self.y)
    }
}

/// Public state of a host-managed load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageLoadStatus {
    Loading,
    Ready,
    Missing,
    Failed,
}

/// A completion capability tied to one ImageContent owner and request generation.
#[derive(Clone, Debug)]
pub struct ImageLoadRequest {
    owner: Arc<()>,
    generation: u64,
}
impl ImageLoadRequest {
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// A successful host completion provides an already registered bitmap lease.
#[derive(Clone, Debug)]
pub enum ImageLoadOutcome {
    Ready {
        lease: BitmapImageLease,
        logical_size: Size,
    },
    Missing,
    Failed(String),
}

/// Invalid public ImageContent configuration or source update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageContentError {
    InvalidLogicalSize,
    InvalidAlternative,
    InvalidAlignment,
    SourceGenerationExhausted,
}

impl std::fmt::Display for ImageContentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid bitmap ImageContent: {self:?}")
    }
}
impl std::error::Error for ImageContentError {}

#[derive(Clone, Debug)]
struct ReadyImage {
    lease: BitmapImageLease,
    logical_size: Size,
}
#[derive(Clone, Debug)]
enum State {
    Loading {
        previous: Option<ReadyImage>,
    },
    Ready(ReadyImage),
    Missing,
    Failed {
        message: String,
        previous: Option<ReadyImage>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct ImageLayout {
    destination: Rect,
    uv: Rect,
}

fn valid_size(size: Size) -> bool {
    size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0
}
fn ready_state(state: &State) -> Option<&ReadyImage> {
    match state {
        State::Ready(image) => Some(image),
        State::Loading {
            previous: Some(image),
        }
        | State::Failed {
            previous: Some(image),
            ..
        } => Some(image),
        State::Loading { previous: None }
        | State::Failed { previous: None, .. }
        | State::Missing => None,
    }
}
fn image_layout(
    frame: Rect,
    source: Size,
    fit: ImageFit,
    alignment: ImageAlignment,
) -> Option<ImageLayout> {
    if !valid_size(source)
        || !alignment.valid()
        || !frame.origin.x.is_finite()
        || !frame.origin.y.is_finite()
        || !frame.size.width.is_finite()
        || !frame.size.height.is_finite()
        || frame.size.width <= 0.0
        || frame.size.height <= 0.0
    {
        return None;
    }
    let fw = f64::from(frame.size.width);
    let fh = f64::from(frame.size.height);
    let sw = f64::from(source.width);
    let sh = f64::from(source.height);
    let destination;
    let uv;
    match fit {
        ImageFit::Contain => {
            let scale = (fw / sw).min(fh / sh);
            let width = (sw * scale) as f32;
            let height = (sh * scale) as f32;
            let x = frame.origin.x + ((fw - f64::from(width)) * f64::from(alignment.x)) as f32;
            let y = frame.origin.y + ((fh - f64::from(height)) * f64::from(alignment.y)) as f32;
            destination = Rect::new(x, y, width, height);
            uv = Rect::new(0.0, 0.0, 1.0, 1.0);
        }
        ImageFit::Cover => {
            let frame_aspect = fw / fh;
            let source_aspect = sw / sh;
            if source_aspect > frame_aspect {
                let width = (frame_aspect / source_aspect) as f32;
                uv = Rect::new((1.0 - width) * alignment.x, 0.0, width, 1.0);
            } else {
                let height = (source_aspect / frame_aspect) as f32;
                uv = Rect::new(0.0, (1.0 - height) * alignment.y, 1.0, height);
            }
            destination = frame;
        }
    }
    (destination.origin.x.is_finite()
        && destination.origin.y.is_finite()
        && destination.size.width.is_finite()
        && destination.size.height.is_finite()
        && uv.origin.x.is_finite()
        && uv.origin.y.is_finite()
        && uv.size.width.is_finite()
        && uv.size.height.is_finite()
        && destination.size.width > 0.0
        && destination.size.height > 0.0
        && uv.size.width > 0.0
        && uv.size.height > 0.0)
        .then_some(ImageLayout { destination, uv })
}

/// A noninteractive Image owner in the ordinary Composition tree.
///
/// Loading, missing and failed states keep their measured logical size. If a
/// source is loading or fails after a prior ready image, the previous lease
/// remains painted until a successful replacement or an explicit Missing
/// result. File/network/decode work belongs to the host. After applying a
/// completion through Content::component_mut, the host schedules its redraw.
#[derive(Debug)]
pub struct ImageContent {
    semantics: ImageSemantics,
    logical_size: Size,
    fit: ImageFit,
    alignment: ImageAlignment,
    state: State,
    source_owner: Arc<()>,
    generation: u64,
    active_request: Option<u64>,
    arranged: Option<ImageLayout>,
}
impl ImageContent {
    pub fn loading(
        logical_size: Size,
        semantics: ImageSemantics,
    ) -> Result<Self, ImageContentError> {
        Self::build(logical_size, semantics, State::Loading { previous: None })
    }
    pub fn ready(
        lease: BitmapImageLease,
        logical_size: Size,
        semantics: ImageSemantics,
    ) -> Result<Self, ImageContentError> {
        Self::build(
            logical_size,
            semantics,
            State::Ready(ReadyImage {
                lease,
                logical_size,
            }),
        )
    }
    fn build(
        logical_size: Size,
        semantics: ImageSemantics,
        state: State,
    ) -> Result<Self, ImageContentError> {
        if !valid_size(logical_size) {
            return Err(ImageContentError::InvalidLogicalSize);
        }
        if matches!(&semantics, ImageSemantics::Alternative(name) if name.trim().is_empty()) {
            return Err(ImageContentError::InvalidAlternative);
        }
        Ok(Self {
            semantics,
            logical_size,
            fit: ImageFit::Contain,
            alignment: ImageAlignment::CENTER,
            state,
            source_owner: Arc::new(()),
            generation: 0,
            active_request: None,
            arranged: None,
        })
    }
    pub fn with_fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self.arranged = None;
        self
    }
    pub fn with_alignment(mut self, alignment: ImageAlignment) -> Result<Self, ImageContentError> {
        if !alignment.valid() {
            return Err(ImageContentError::InvalidAlignment);
        }
        self.alignment = alignment;
        self.arranged = None;
        Ok(self)
    }
    pub fn set_accessibility(
        &mut self,
        semantics: ImageSemantics,
    ) -> Result<(), ImageContentError> {
        if matches!(&semantics, ImageSemantics::Alternative(name) if name.trim().is_empty()) {
            return Err(ImageContentError::InvalidAlternative);
        }
        self.semantics = semantics;
        Ok(())
    }
    pub fn status(&self) -> ImageLoadStatus {
        match &self.state {
            State::Loading { .. } => ImageLoadStatus::Loading,
            State::Ready(_) => ImageLoadStatus::Ready,
            State::Missing => ImageLoadStatus::Missing,
            State::Failed { .. } => ImageLoadStatus::Failed,
        }
    }
    pub fn failure_message(&self) -> Option<&str> {
        match &self.state {
            State::Failed { message, .. } => Some(message),
            _ => None,
        }
    }
    pub fn logical_size(&self) -> Size {
        self.logical_size
    }

    /// Start a newer host request. Its completion token cannot be used by a
    /// different ImageContent instance or by an earlier source generation.
    pub fn begin_load(&mut self) -> Result<ImageLoadRequest, ImageContentError> {
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(ImageContentError::SourceGenerationExhausted)?;
        self.generation = generation;
        self.active_request = Some(generation);
        let previous = ready_state(&self.state).cloned();
        self.state = State::Loading { previous };
        self.arranged = None;
        Ok(ImageLoadRequest {
            owner: Arc::clone(&self.source_owner),
            generation,
        })
    }
    /// Apply one current completion. Replayed, stale, cross-component and
    /// A→B→A completions return false and leave the current image untouched.
    pub fn complete(
        &mut self,
        request: ImageLoadRequest,
        outcome: ImageLoadOutcome,
    ) -> Result<bool, ImageContentError> {
        if !Arc::ptr_eq(&request.owner, &self.source_owner)
            || self.active_request != Some(request.generation)
            || self.generation != request.generation
        {
            return Ok(false);
        }
        let previous = ready_state(&self.state).cloned();
        match outcome {
            ImageLoadOutcome::Ready {
                lease,
                logical_size,
            } => {
                if !valid_size(logical_size) {
                    return Err(ImageContentError::InvalidLogicalSize);
                }
                self.logical_size = logical_size;
                self.state = State::Ready(ReadyImage {
                    lease,
                    logical_size,
                });
            }
            ImageLoadOutcome::Missing => self.state = State::Missing,
            ImageLoadOutcome::Failed(message) => self.state = State::Failed { message, previous },
        }
        self.active_request = None;
        self.arranged = None;
        Ok(true)
    }
    // Query the original resource owner without duplicating its load state.
    pub(super) fn displayed_logical_size(&self) -> Option<Size> {
        ready_state(&self.state).map(|image| image.logical_size)
    }
    fn arranged_ready(&self) -> Option<&ReadyImage> {
        ready_state(&self.state)
    }
}
impl Component for ImageContent {
    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Local
    }

    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let width = if available.width.is_finite() {
            available.width.max(0.0)
        } else {
            self.logical_size.width
        };
        let height = if available.height.is_finite() {
            available.height.max(0.0)
        } else {
            self.logical_size.height
        };
        MeasuredChild::new(Size::new(
            self.logical_size.width.min(width),
            self.logical_size.height.min(height),
        ))
    }
    fn arrange(&mut self, _text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.arranged = if frame.intersection(&clip).is_some() {
            self.arranged_ready()
                .and_then(|image| image_layout(frame, image.logical_size, self.fit, self.alignment))
        } else {
            None
        };
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        _frame: Rect,
        _clip: Rect,
        _theme: ContentTheme,
    ) {
        let (Some(image), Some(layout)) = (self.arranged_ready(), self.arranged) else {
            return;
        };
        let quad = ImageQuad {
            rect: layout.destination,
            image: image.lease.id(),
            opacity: 1.0,
        };
        let mut transformed = TransformedImageQuad::new(quad, 0.0);
        transformed.uv = layout.uv;
        scene.push_controlled_transformed_image(transformed, image.lease.scene_token());
    }
    fn semantics(&self) -> ContentSemantics {
        self.semantics.content_semantics()
    }
    fn semantic_hidden(&self) -> bool {
        matches!(&self.semantics, ImageSemantics::Decorative)
    }
    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            // A completion belongs to the mount that issued its request. The
            // Content may be returned and appended again, so generation alone
            // is not enough to distinguish that new mount.
            self.source_owner = Arc::new(());
            self.active_request = None;
            self.arranged = None;
        }
    }
    fn validate(&self) -> Result<(), CompositionError> {
        if matches!(&self.semantics, ImageSemantics::Alternative(name) if name.trim().is_empty()) {
            Err(CompositionError::EmptyAccessibleName)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn load_requests_reject_stale_replay_and_other_component_owners() {
        let mut first = ImageContent::loading(
            Size::new(2.0, 1.0),
            ImageSemantics::Alternative("Chart".into()),
        )
        .unwrap();
        let mut second =
            ImageContent::loading(Size::new(2.0, 1.0), ImageSemantics::Decorative).unwrap();
        let older = first.begin_load().unwrap();
        let newer = first.begin_load().unwrap();
        assert!(
            !first
                .complete(older.clone(), ImageLoadOutcome::Missing)
                .unwrap()
        );
        assert!(
            !second
                .complete(newer.clone(), ImageLoadOutcome::Missing)
                .unwrap()
        );
        assert!(
            first
                .complete(newer.clone(), ImageLoadOutcome::Missing)
                .unwrap()
        );
        assert_eq!(first.status(), ImageLoadStatus::Missing);
        assert!(
            !first
                .complete(newer, ImageLoadOutcome::Failed("late replay".into()))
                .unwrap()
        );
    }
    #[test]
    fn invalid_dimensions_alignment_and_alt_fail_closed() {
        assert!(matches!(
            ImageContent::loading(Size::new(f32::NAN, 1.0), ImageSemantics::Decorative),
            Err(ImageContentError::InvalidLogicalSize)
        ));
        assert!(matches!(
            ImageContent::loading(
                Size::new(1.0, 1.0),
                ImageSemantics::Alternative(" \n".into())
            ),
            Err(ImageContentError::InvalidAlternative)
        ));
        assert!(matches!(
            ImageContent::loading(Size::new(1.0, 1.0), ImageSemantics::Decorative)
                .unwrap()
                .with_alignment(ImageAlignment::new(2.0, 0.5)),
            Err(ImageContentError::InvalidAlignment)
        ));
    }
    #[test]
    fn contain_preserves_source_and_cover_crops_according_to_alignment() {
        let contain = image_layout(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Size::new(200.0, 100.0),
            ImageFit::Contain,
            ImageAlignment::CENTER,
        )
        .unwrap();
        assert_eq!(contain.destination, Rect::new(0.0, 25.0, 100.0, 50.0));
        assert_eq!(contain.uv, Rect::new(0.0, 0.0, 1.0, 1.0));
        let cover = image_layout(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Size::new(200.0, 100.0),
            ImageFit::Cover,
            ImageAlignment::new(1.0, 0.5),
        )
        .unwrap();
        assert_eq!(cover.destination, Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(cover.uv, Rect::new(0.5, 0.0, 0.5, 1.0));
    }
}
