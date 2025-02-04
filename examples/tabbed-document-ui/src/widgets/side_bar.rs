use cushy::figures::units::{Lp, Px};
use cushy::localize;
use cushy::styles::ContainerLevel;
use cushy::reactive::value::{Dynamic, Switchable};
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::widgets::{Grid, Space};
use cushy::widgets::grid::{GridDimension, GridWidgets};
use cushy::widgets::label::{Displayable, LabelOverflow};

pub struct SideBarItem {
    label: WidgetInstance,
    field: WidgetInstance,
}

#[derive(Default)]
pub struct SideBar {
    items: Vec<SideBarItem>,
    grid_dimensions: Dynamic<[GridDimension;2]>
}

impl SideBar {
    pub fn with_fixed_width_columns(self) -> Self {
        Self {
            items: self.items,
            grid_dimensions: Dynamic::new([
                // label
                GridDimension::Measured { size: Lp::new(100).into() },
                // value
                GridDimension::Measured { size: Lp::new(150).into() }
            ]),
        }
    }

    pub fn push(&mut self, item: SideBarItem) {
        self.items.push(item);
    }

    pub fn make_widget(&self) -> WidgetInstance {

        let grid_rows: Vec<(WidgetInstance, WidgetInstance)> = self.items.iter().map(|item|{
            (
                item.label.clone(),
                item.field.clone()
            )
        }).collect();

        let grid_row_widgets = GridWidgets::from(grid_rows);

        let grid = Grid::from_rows(grid_row_widgets);

        let grid_widget = grid
            .dimensions(self.grid_dimensions.clone())
            .align_top()
            .make_widget();

        let scrollable_content = grid_widget
            .vertical_scroll()
            .contain_level(ContainerLevel::High)
            .expand_vertically()
            .make_widget();

        let sidebar_header = localize!("side-bar-header")
            .centered()
            .align_left()
            .contain_level(ContainerLevel::Highest);

        let sidebar_footer = localize!("side-bar-footer")
            .centered()
            .align_left()
            .contain_level(ContainerLevel::Highest);

        let sidebar_widget = sidebar_header
            .and(scrollable_content)
            .and(sidebar_footer)
            .into_rows()
            .gutter(Px::new(0))
            // required so that when the background of the sidebar fills the container
            .expand_vertically()
            .contain_level(ContainerLevel::Mid)
            .make_widget();

        sidebar_widget
    }
}

impl SideBarItem {
    pub fn from_field(label: impl MakeWidget, field: impl MakeWidget) -> Self {
        Self {
            label: label.make_widget(),
            field: field.make_widget(),
        }
    }

    // FIXME rename to from_optional_value
    pub fn new(label: impl MakeWidget, value: Dynamic<Option<String>>) -> Self {
        let field = value.clone().switcher({
            move |value, _| {
                match value.clone() {
                    Some(value) =>
                        value
                            .into_label()
                            .overflow(LabelOverflow::Clip)
                            .make_widget()
                    ,
                    None =>
                        Space::clear()
                            .make_widget(),
                }
            }
        })
            .align_left()
            .make_widget();

        Self {
            label: label.make_widget(),
            field,
        }
    }
}

