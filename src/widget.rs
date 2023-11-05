//! Types for creating reusable widgets (aka components or views).

use std::any::Any;
use std::clone::Clone;
use std::fmt::Debug;
use std::ops::{ControlFlow, Deref};
use std::panic::UnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Rect, Size};

use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext};
use crate::styles::Styles;
use crate::tree::{Tree, WidgetId};
use crate::value::{IntoValue, Value};
use crate::widgets::Style;
use crate::window::{RunningWindow, Window, WindowBehavior};
use crate::{ConstraintLimit, Run};

/// A type that makes up a graphical user interface.
///
/// This type can go by many names in other UI frameworks: View, Component,
/// Control.
pub trait Widget: Send + UnwindSafe + Debug + 'static {
    /// Redraw the contents of this widget.
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>);

    /// Layout this widget and returns the ideal size based on its contents and
    /// the `available_space`.
    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx>;

    /// The widget has been mounted into a parent widget.
    #[allow(unused_variables)]
    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget has been removed from its parent widget.
    #[allow(unused_variables)]
    fn unmounted(&mut self, context: &mut EventContext<'_, '_>) {}

    /// Returns true if this widget should respond to mouse input at `location`.
    #[allow(unused_variables)]
    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) -> bool {
        false
    }

    /// The widget is currently has a cursor hovering it at `location`.
    #[allow(unused_variables)]
    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) {}

    /// The widget is no longer being hovered.
    #[allow(unused_variables)]
    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {}

    /// This widget has been targeted to be focused. If this function returns
    /// true, the widget will be focused. If false, Gooey will continue
    /// searching for another focus target.
    #[allow(unused_variables)]
    fn accept_focus(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        false
    }

    /// The widget has received focus for user input.
    #[allow(unused_variables)]
    fn focus(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget is no longer focused for user input.
    #[allow(unused_variables)]
    fn blur(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget has become the active widget.
    #[allow(unused_variables)]
    fn activate(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget is no longer active.
    #[allow(unused_variables)]
    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {}

    /// A mouse button event has occurred at `location`. Returns whether the
    /// event has been handled or not.
    ///
    /// If an event is handled, the widget will receive callbacks for
    /// [`mouse_drag`](Self::mouse_drag) and [`mouse_up`](Self::mouse_up).
    #[allow(unused_variables)]
    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        IGNORED
    }

    /// A mouse button is being held down as the cursor is moved across the
    /// widget.
    #[allow(unused_variables)]
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
    }

    /// A mouse button is no longer being pressed.
    #[allow(unused_variables)]
    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
    }

    /// A keyboard event has been sent to this widget. Returns whether the event
    /// has been handled or not.
    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        IGNORED
    }

    /// An input manager event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_, '_>) -> EventHandling {
        IGNORED
    }

    /// A mouse wheel event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        IGNORED
    }
}

impl<T> Run for T
where
    T: MakeWidget,
{
    fn run(self) -> crate::Result {
        self.make_widget().run()
    }
}

/// A type that can create a widget.
pub trait MakeWidget: Sized {
    /// Returns a new widget.
    fn make_widget(self) -> WidgetInstance;

    /// Associates `styles` with this widget.
    ///
    /// This is equivalent to `Style::new(styles, self)`.
    fn with_styles(self, styles: impl Into<Styles>) -> Style
    where
        Self: Sized,
    {
        Style::new(styles, self)
    }

    /// Sets the widget that should be focused next.
    ///
    /// Gooey automatically determines reverse tab order by using this same
    /// relationship.
    fn with_next_focus(self, next_focus: impl IntoValue<Option<WidgetInstance>>) -> WidgetInstance {
        self.make_widget().with_next_focus(next_focus)
    }
}

impl<T> MakeWidget for T
where
    T: Widget,
{
    fn make_widget(self) -> WidgetInstance {
        WidgetInstance::new(self)
    }
}

impl MakeWidget for WidgetInstance {
    fn make_widget(self) -> WidgetInstance {
        self
    }
}

/// A type that represents whether an event has been handled or ignored.
pub type EventHandling = ControlFlow<EventHandled, EventIgnored>;

/// A marker type that represents a handled event.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]

pub struct EventHandled;
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// A marker type that represents an ignored event.
pub struct EventIgnored;

/// An [`EventHandling`] value that represents a handled event.
pub const HANDLED: EventHandling = EventHandling::Break(EventHandled);

/// An [`EventHandling`] value that represents an ignored event.
pub const IGNORED: EventHandling = EventHandling::Continue(EventIgnored);

pub(crate) trait AnyWidget: Widget {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> AnyWidget for T
where
    T: Widget,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// An instance of a [`Widget`].
#[derive(Clone, Debug)]
pub struct WidgetInstance {
    widget: Arc<Mutex<dyn AnyWidget>>,
    next_focus: Value<Option<Arc<Mutex<dyn AnyWidget>>>>,
}

impl WidgetInstance {
    /// Returns a new instance containing `widget`.
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self {
            widget: Arc::new(Mutex::new(widget)),
            next_focus: Value::default(),
        }
    }

    /// Sets the widget that should be focused next.
    ///
    /// Gooey automatically determines reverse tab order by using this same
    /// relationship.
    #[must_use]
    pub fn with_next_focus(
        mut self,
        next_focus: impl IntoValue<Option<WidgetInstance>>,
    ) -> WidgetInstance {
        self.next_focus = match next_focus.into_value() {
            Value::Constant(maybe_widget) => {
                Value::Constant(maybe_widget.map(|widget| widget.widget))
            }
            Value::Dynamic(dynamic) => Value::Dynamic(
                dynamic
                    .map_each(|instance| instance.as_ref().map(|instance| instance.widget.clone())),
            ),
        };
        self
    }

    /// Locks the widget for exclusive access. Locking widgets should only be
    /// done for brief moments of time when you are certain no deadlocks can
    /// occur due to other widget locks being held.
    pub fn lock(&self) -> WidgetGuard<'_> {
        WidgetGuard(
            self.widget
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g),
        )
    }

    /// Runs this widget instance as an application.
    pub fn run(self) -> crate::Result {
        Window::<WidgetInstance>::new(self).run()
    }
}

impl Eq for WidgetInstance {}

impl PartialEq for WidgetInstance {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.widget, &other.widget)
    }
}

impl WindowBehavior for WidgetInstance {
    type Context = Self;

    fn initialize(_window: &mut RunningWindow<'_>, context: Self::Context) -> Self {
        context
    }

    fn make_root(&mut self) -> WidgetInstance {
        self.clone()
    }
}

/// A function that can be invoked with a parameter (`T`) and returns `R`.
///
/// This type is used by widgets to signal various events.
pub struct Callback<T = (), R = ()>(Box<dyn CallbackFunction<T, R>>);

impl<T, R> Debug for Callback<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Callback")
            .field(&(self as *const Self))
            .finish()
    }
}

impl<T, R> Callback<T, R> {
    /// Returns a new instance that calls `function` each time the callback is
    /// invoked.
    pub fn new<F>(function: F) -> Self
    where
        F: FnMut(T) -> R + Send + UnwindSafe + 'static,
    {
        Self(Box::new(function))
    }

    /// Invokes the wrapped function and returns the produced value.
    pub fn invoke(&mut self, value: T) -> R {
        self.0.invoke(value)
    }
}

trait CallbackFunction<T, R>: Send + UnwindSafe {
    fn invoke(&mut self, value: T) -> R;
}

impl<T, R, F> CallbackFunction<T, R> for F
where
    F: FnMut(T) -> R + Send + UnwindSafe,
{
    fn invoke(&mut self, value: T) -> R {
        self(value)
    }
}

/// A [`Widget`] that has been attached to a widget hierarchy.
#[derive(Clone)]
pub struct ManagedWidget {
    pub(crate) id: WidgetId,
    pub(crate) widget: WidgetInstance,
    pub(crate) tree: Tree,
}

impl Debug for ManagedWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedWidget")
            .field("id", &self.id)
            .field("widget", &self.widget)
            .finish_non_exhaustive()
    }
}

impl ManagedWidget {
    /// Locks the widget for exclusive access. Locking widgets should only be
    /// done for brief moments of time when you are certain no deadlocks can
    /// occur due to other widget locks being held.
    #[must_use]
    pub fn lock(&self) -> WidgetGuard<'_> {
        self.widget.lock()
    }

    pub(crate) fn set_layout(&self, rect: Rect<Px>) {
        self.tree.set_layout(self.id, rect);
    }

    /// Returns the region that the widget was last rendered at.
    #[must_use]
    pub fn last_layout(&self) -> Option<Rect<Px>> {
        self.tree.layout(self.id)
    }

    /// Returns true if this widget is the currently active widget.
    #[must_use]
    pub fn active(&self) -> bool {
        self.tree.active_widget() == Some(self.id)
    }

    /// Returns true if this widget is currently the hovered widget.
    #[must_use]
    pub fn hovered(&self) -> bool {
        self.tree.is_hovered(self.id)
    }

    /// Returns true if this widget that is directly beneath the cursor.
    #[must_use]
    pub fn primary_hover(&self) -> bool {
        self.tree.hovered_widget() == Some(self.id)
    }

    /// Returns true if this widget is the currently focused widget.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.tree.focused_widget() == Some(self.id)
    }

    /// Returns the parent of this widget.
    #[must_use]
    pub fn parent(&self) -> Option<ManagedWidget> {
        self.tree.parent(self.id).map(|id| self.tree.widget(id))
    }

    pub(crate) fn attach_styles(&self, styles: Styles) {
        self.tree.attach_styles(self.id, styles);
    }

    pub(crate) fn reset_child_layouts(&self) {
        self.tree.reset_child_layouts(self.id);
    }
}

impl PartialEq for ManagedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.widget == other.widget
    }
}

impl PartialEq<WidgetInstance> for ManagedWidget {
    fn eq(&self, other: &WidgetInstance) -> bool {
        &self.widget == other
    }
}

/// Exclusive access to a widget.
///
/// This type is powered by a `Mutex`, which means care must be taken to prevent
/// deadlocks.
pub struct WidgetGuard<'a>(MutexGuard<'a, dyn AnyWidget>);

impl WidgetGuard<'_> {
    pub(crate) fn as_widget(&mut self) -> &mut dyn AnyWidget {
        &mut *self.0
    }

    /// Returns a reference to `T` if it is the type contained.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0.as_any().downcast_ref()
    }

    /// Returns an exclusive reference to `T` if it is the type contained.
    #[must_use]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.0.as_any_mut().downcast_mut()
    }
}

/// A list of [`Widget`]s.
#[derive(Debug, Default)]
#[must_use]
pub struct Children {
    ordered: Vec<WidgetInstance>,
}

impl Children {
    /// Returns an empty list.
    pub const fn new() -> Self {
        Self {
            ordered: Vec::new(),
        }
    }

    /// Returns a list with enough capacity to hold `capacity` widgets without
    /// reallocation.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            ordered: Vec::with_capacity(capacity),
        }
    }

    /// Pushes `widget` into the list.
    pub fn push<W>(&mut self, widget: W)
    where
        W: MakeWidget,
    {
        self.ordered.push(widget.make_widget());
    }

    /// Adds `widget` to self and returns the updated list.
    pub fn with_widget<W>(mut self, widget: W) -> Self
    where
        W: MakeWidget,
    {
        self.push(widget);
        self
    }

    /// Returns the number of widgets in this list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    /// Returns true if there are no widgets in this list.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }
}

impl<W> FromIterator<W> for Children
where
    W: MakeWidget,
{
    fn from_iter<T: IntoIterator<Item = W>>(iter: T) -> Self {
        Self {
            ordered: iter.into_iter().map(MakeWidget::make_widget).collect(),
        }
    }
}

impl Deref for Children {
    type Target = [WidgetInstance];

    fn deref(&self) -> &Self::Target {
        &self.ordered
    }
}

/// A child widget
#[derive(Debug, Clone)]
pub enum WidgetRef {
    /// An unmounted child widget
    Unmounted(WidgetInstance),
    /// A mounted child widget
    Mounted(ManagedWidget),
}

impl WidgetRef {
    /// Returns a new unmounted child
    pub fn new(widget: impl MakeWidget) -> Self {
        Self::Unmounted(widget.make_widget())
    }

    /// Returns this child, mounting it in the process if necessary.
    pub fn mounted(&mut self, context: &mut EventContext<'_, '_>) -> ManagedWidget {
        if let WidgetRef::Unmounted(instance) = self {
            *self = WidgetRef::Mounted(context.push_child(instance.clone()));
        }

        let Self::Mounted(widget) = self else {
            unreachable!("just initialized")
        };
        widget.clone()
    }
}
