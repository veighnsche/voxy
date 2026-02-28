use gtk4::{
    prelude::*, ApplicationWindow, Button, CheckButton, ComboBoxText, DropDown, Entry, PickFlags,
    Scale, ScrolledWindow, SpinButton, Switch, TextView, ToggleButton, Widget,
};

pub fn should_start_drag(window: &ApplicationWindow, x: f64, y: f64) -> bool {
    let Some(target) = window.pick(x, y, PickFlags::DEFAULT) else {
        return true;
    };

    !is_interactive_target(target)
}

fn is_interactive_target(mut widget: Widget) -> bool {
    loop {
        if widget.is::<ApplicationWindow>() {
            return false;
        }

        if widget.is::<Button>()
            || widget.is::<ToggleButton>()
            || widget.is::<CheckButton>()
            || widget.is::<Switch>()
            || widget.is::<DropDown>()
            || widget.is::<ComboBoxText>()
            || widget.is::<Entry>()
            || widget.is::<TextView>()
            || widget.is::<ScrolledWindow>()
            || widget.is::<Scale>()
            || widget.is::<SpinButton>()
        {
            return true;
        }

        let Some(parent) = widget.parent() else {
            return false;
        };
        widget = parent;
    }
}
