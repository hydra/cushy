use cushy::widget::{MakeWidget};
use cushy::widgets::pile::Pile;
use cushy::{App, Run};
use cushy::widgets::label::Displayable;
use cushy::window::PendingWindow;

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {
    let pending = PendingWindow::default();

    let pile = Pile::default();

    let handle = pile.push("show a pile!".to_label());
    handle.show(false);

    let ui = pending.with_root(
        pile.centered().expand()
    );

    ui.open_centered(app)?;

    Ok(())
}
