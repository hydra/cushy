use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, UnwindSafe};

use kludgine::app::winit::dpi::PhysicalPosition;
use kludgine::app::winit::error::EventLoopError;
use kludgine::app::winit::event::{
    DeviceId, ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::app::winit::keyboard::KeyCode;
use kludgine::app::WindowBehavior as _;
use kludgine::figures::units::Px;
use kludgine::figures::Point;
use kludgine::render::Drawing;
use kludgine::Kludgine;

use crate::context::{EventContext, Exclusive, GraphicsContext, WidgetContext};
use crate::graphics::Graphics;
use crate::styles::Styles;
use crate::tree::Tree;
use crate::utils::ModifiersExt;
use crate::widget::{BoxedWidget, EventHandling, ManagedWidget, Widget, HANDLED, UNHANDLED};
use crate::window::sealed::WindowCommand;

pub type RunningWindow<'window> = kludgine::app::Window<'window, WindowCommand>;
pub type WindowAttributes = kludgine::app::WindowAttributes<WindowCommand>;

#[must_use]
pub struct Window<Behavior>
where
    Behavior: WindowBehavior,
{
    context: Behavior::Context,
    pub attributes: WindowAttributes,
    pub styles: Option<Styles>,
}

impl<Behavior> Default for Window<Behavior>
where
    Behavior: WindowBehavior,
    Behavior::Context: Default,
{
    fn default() -> Self {
        let context = Behavior::Context::default();
        Self::new(context)
    }
}

impl Window<BoxedWidget> {
    pub fn for_widget<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self::new(BoxedWidget::new(widget))
    }
}

impl<Behavior> Window<Behavior>
where
    Behavior: WindowBehavior,
{
    pub fn new(context: Behavior::Context) -> Self {
        Self {
            attributes: WindowAttributes {
                title: String::from("Gooey App"),
                ..WindowAttributes::default()
            },
            context,
            styles: None,
        }
    }

    pub fn styles(mut self, styles: Styles) -> Self {
        self.styles = Some(styles);
        self
    }

    pub fn run(self) -> Result<(), EventLoopError> {
        GooeyWindow::<Behavior>::run_with(AssertUnwindSafe((
            self.context,
            RefCell::new(WindowSettings {
                styles: self.styles,
                attributes: Some(self.attributes),
            }),
        )))
    }
}

pub trait WindowBehavior: Sized + 'static {
    type Context: UnwindSafe + Send + 'static;

    fn initialize(window: &mut RunningWindow<'_>, context: Self::Context) -> Self;

    fn make_root(&mut self) -> BoxedWidget;

    #[allow(unused_variables)]
    fn close_requested(&self, window: &mut RunningWindow<'_>) -> bool {
        true
    }

    fn run() -> Result<(), EventLoopError>
    where
        Self::Context: Default,
    {
        Self::run_with(<Self::Context>::default())
    }

    fn run_with(context: Self::Context) -> Result<(), EventLoopError> {
        Window::<Self>::new(context).run()
    }
}

struct GooeyWindow<T> {
    behavior: T,
    root: ManagedWidget,
    contents: Drawing,
    should_close: bool,
    mouse_state: MouseState,
}

impl<T> GooeyWindow<T>
where
    T: WindowBehavior,
{
    fn request_close(&mut self, window: &mut RunningWindow<'_>) -> bool {
        self.should_close |= self.behavior.close_requested(window);

        self.should_close
    }
}

impl<T> kludgine::app::WindowBehavior<WindowCommand> for GooeyWindow<T>
where
    T: WindowBehavior,
{
    type Context = AssertUnwindSafe<(T::Context, RefCell<WindowSettings>)>;

    fn initialize(
        mut window: RunningWindow<'_>,
        _graphics: &mut kludgine::Graphics<'_>,
        context: Self::Context,
    ) -> Self {
        let mut behavior = T::initialize(&mut window, context.0 .0);
        let root = Tree::default().push_boxed(behavior.make_root(), None);
        if let Some(styles) = context.0 .1.borrow_mut().styles.take() {
            root.attach_styles(styles);
        }
        Self {
            behavior,
            root,
            contents: Drawing::default(),
            should_close: false,
            mouse_state: MouseState {
                location: None,
                widget: None,
                devices: HashMap::default(),
            },
        }
    }

    fn prepare(&mut self, mut window: RunningWindow<'_>, graphics: &mut kludgine::Graphics<'_>) {
        graphics.reset_text_attributes();
        self.root.tree.reset_render_order();
        let graphics = self.contents.new_frame(graphics);
        GraphicsContext {
            widget: WidgetContext::new(&self.root, &mut window),
            graphics: Exclusive::Owned(Graphics::new(graphics)),
        }
        .redraw();
    }

    fn render<'pass>(
        &'pass mut self,
        _window: RunningWindow<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        self.contents.render(graphics);

        !self.should_close
    }

    fn initial_window_attributes(
        context: &Self::Context,
    ) -> kludgine::app::WindowAttributes<WindowCommand> {
        context
            .1
            .borrow_mut()
            .attributes
            .take()
            .expect("called more than once")
    }

    fn close_requested(&mut self, mut window: RunningWindow<'_>, _kludgine: &mut Kludgine) -> bool {
        self.request_close(&mut window)
    }

    // fn power_preference() -> wgpu::PowerPreference {
    //     wgpu::PowerPreference::default()
    // }

    // fn limits(adapter_limits: wgpu::Limits) -> wgpu::Limits {
    //     wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits)
    // }

    // fn clear_color() -> Option<kludgine::Color> {
    //     Some(kludgine::Color::BLACK)
    // }

    // fn focus_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn occlusion_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn scale_factor_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn resized(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn theme_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn dropped_file(&mut self, window: kludgine::app::Window<'_, ()>, path: std::path::PathBuf) {}

    // fn hovered_file(&mut self, window: kludgine::app::Window<'_, ()>, path: std::path::PathBuf) {}

    // fn hovered_file_cancelled(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn received_character(&mut self, window: kludgine::app::Window<'_, ()>, char: char) {}

    fn keyboard_input(
        &mut self,
        mut window: RunningWindow<'_>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) {
        let target = self.root.tree.focused_widget().unwrap_or(self.root.id);
        let target = self.root.tree.widget(target);
        let mut target = EventContext::new(WidgetContext::new(&target, &mut window), kludgine);

        let handled = recursively_handle_event(&mut target, |widget| {
            widget.keyboard_input(device_id, input.clone(), is_synthetic)
        })
        .is_some();
        drop(target);

        if !handled && !input.state.is_pressed() {
            match input.physical_key {
                KeyCode::KeyW if window.modifiers().state().primary() => {
                    if self.request_close(&mut window) {
                        window.set_needs_redraw();
                    }
                }
                _ => {}
            }
        }
    }

    fn mouse_wheel(
        &mut self,
        mut window: RunningWindow<'_>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) {
        let widget = self.root.tree.hovered_widget().unwrap_or(self.root.id);

        let widget = self.root.tree.widget(widget);
        let mut widget = EventContext::new(WidgetContext::new(&widget, &mut window), kludgine);
        recursively_handle_event(&mut widget, |widget| {
            widget.mouse_wheel(device_id, delta, phase)
        });
    }

    // fn modifiers_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    fn ime(&mut self, mut window: RunningWindow<'_>, kludgine: &mut Kludgine, ime: Ime) {
        let target = self.root.tree.focused_widget().unwrap_or(self.root.id);
        let target = self.root.tree.widget(target);
        let mut target = EventContext::new(WidgetContext::new(&target, &mut window), kludgine);

        let _handled =
            recursively_handle_event(&mut target, |widget| widget.ime(ime.clone())).is_some();
    }

    fn cursor_moved(
        &mut self,
        mut window: RunningWindow<'_>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
        let location = Point::<Px>::from(position);
        self.mouse_state.location = Some(location);

        if let Some(state) = self.mouse_state.devices.get(&device_id) {
            // Mouse Drag
            for (button, handler) in state {
                let mut context =
                    EventContext::new(WidgetContext::new(handler, &mut window), kludgine);
                let last_rendered_at = context.last_rendered_at().expect("passed hit test");
                context.mouse_drag(location - last_rendered_at.origin, device_id, *button);
            }
        } else {
            // Hover
            let mut context =
                EventContext::new(WidgetContext::new(&self.root, &mut window), kludgine);
            self.mouse_state.widget = None;
            for widget in self.root.tree.widgets_at_point(location) {
                let mut widget_context = context.for_other(&widget);
                let relative = location
                    - widget_context
                        .last_rendered_at()
                        .expect("passed hit test")
                        .origin;

                if widget_context.hit_test(relative) {
                    widget_context.hover(relative);
                    drop(widget_context);
                    self.mouse_state.widget = Some(widget);
                    break;
                }
            }

            if self.mouse_state.widget.is_none() {
                context.clear_hover();
            }
        }
    }

    fn cursor_left(
        &mut self,
        mut window: RunningWindow<'_>,
        kludgine: &mut Kludgine,
        _device_id: DeviceId,
    ) {
        if self.mouse_state.widget.take().is_some() {
            let mut context =
                EventContext::new(WidgetContext::new(&self.root, &mut window), kludgine);
            context.clear_hover();
        }
    }

    fn mouse_input(
        &mut self,
        mut window: RunningWindow<'_>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
        match state {
            ElementState::Pressed => {
                WidgetContext::new(&self.root, &mut window).clear_focus();

                if let (ElementState::Pressed, Some(location), Some(hovered)) =
                    (state, &self.mouse_state.location, &self.mouse_state.widget)
                {
                    if let Some(handler) = recursively_handle_event(
                        &mut EventContext::new(WidgetContext::new(hovered, &mut window), kludgine),
                        |context| {
                            let relative = *location
                                - context.last_rendered_at().expect("passed hit test").origin;
                            context.mouse_down(relative, device_id, button)
                        },
                    ) {
                        self.mouse_state
                            .devices
                            .entry(device_id)
                            .or_default()
                            .insert(button, handler);
                    }
                }
            }
            ElementState::Released => {
                let Some(device_buttons) = self.mouse_state.devices.get_mut(&device_id) else {
                    return;
                };
                let Some(handler) = device_buttons.remove(&button) else {
                    return;
                };
                if device_buttons.is_empty() {
                    self.mouse_state.devices.remove(&device_id);
                }

                let mut context =
                    EventContext::new(WidgetContext::new(&handler, &mut window), kludgine);

                let relative = if let (Some(last_rendered), Some(location)) =
                    (context.last_rendered_at(), self.mouse_state.location)
                {
                    Some(location - last_rendered.origin)
                } else {
                    None
                };

                context.mouse_up(relative, device_id, button);
            }
        }
    }

    fn event(
        &mut self,
        mut window: RunningWindow<'_>,
        _kludgine: &mut Kludgine,
        event: WindowCommand,
    ) {
        match event {
            WindowCommand::Redraw => {
                window.set_needs_redraw();
            }
        }
    }
}

fn recursively_handle_event(
    context: &mut EventContext<'_, '_>,
    mut each_widget: impl FnMut(&mut EventContext<'_, '_>) -> EventHandling,
) -> Option<ManagedWidget> {
    match each_widget(context) {
        HANDLED => Some(context.widget().clone()),
        UNHANDLED => context.parent().and_then(|parent| {
            recursively_handle_event(&mut context.for_other(&parent), each_widget)
        }),
    }
}

pub struct WindowSettings {
    styles: Option<Styles>,
    attributes: Option<WindowAttributes>,
}

#[derive(Default)]
struct MouseState {
    location: Option<Point<Px>>,
    widget: Option<ManagedWidget>,
    devices: HashMap<DeviceId, HashMap<MouseButton, ManagedWidget>>,
}

pub(crate) mod sealed {
    pub enum WindowCommand {
        Redraw,
        // RequestClose,
    }
}
