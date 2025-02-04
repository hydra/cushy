//! The tabs for the application.

use cushy::reactive::value::Dynamic;
use cushy::widget::{WidgetInstance};
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::home::HomeTab;
use crate::app_tabs::new::NewTab;
use crate::context::Context;
use crate::widgets::tab_bar::Tab;

pub mod document;
pub mod home;
pub mod new;

#[derive(Clone)]
pub enum TabKind {
    Home(HomeTab),
    Document(DocumentTab),
    New(NewTab),
}

impl Tab for TabKind {
    fn label(&self, context: &Dynamic<Context>) -> String {
        match self {
            TabKind::Home(tab) => tab.label(context),
            TabKind::Document(tab) => tab.label(context),
            TabKind::New(tab) => tab.label(context),
        }
    }

    fn make_content(&self, context: &Dynamic<Context>) -> WidgetInstance {
        match self {
            TabKind::Home(tab) => tab.make_content(context),
            TabKind::Document(tab) => tab.make_content(context),
            TabKind::New(tab) => tab.make_content(context),
        }
    }
}