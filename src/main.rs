

use jete::terminal::terminal_display;
use jete::editor;


fn main() {

    let (display, inputs) = terminal_display();
    
    editor::run(display, inputs)
}
