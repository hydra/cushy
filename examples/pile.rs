use cushy::widget::{MakeWidget};
use cushy::widgets::pile::Pile;
use cushy::Run;
use cushy::widgets::label::Displayable;

fn main() -> cushy::Result {
    let pile = Pile::default();

    let handle = pile.push("show a pile!".to_label());
    handle.show(false);

    pile.centered().expand()
        .run()
}
