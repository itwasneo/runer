mod desktop;

use gdk4::cairo::Antialias;
use gtk::{prelude::*, DrawingArea, HeaderBar, Label, Notebook, Widget};
use gtk::{Application, ApplicationWindow, Box};
use gtk4 as gtk;
// use log::{error, info};

use self::desktop::console::create_console;

pub fn build_ui(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title("runer")
        .default_height(600)
        .default_width(800)
        .show_menubar(true)
        .build();

    let header = HeaderBar::builder().build();
    window.set_titlebar(Some(&header));

    let gtk_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let gtk_notebook = Notebook::builder()
        .tab_pos(gtk::PositionType::Bottom)
        .show_border(false)
        .build();

    let console = create_console();

    let da = DrawingArea::new();
    da.set_draw_func(draw_axis);

    create_tab(&gtk_notebook, "Logs", &console);
    create_tab(&gtk_notebook, "tab_2", &da);
    create_tab(&gtk_notebook, "tab_3", &Label::new(Some("tab_3_content")));

    gtk_box.append(&gtk_notebook);

    window.set_child(Some(&gtk_box));

    window.present();
}

fn create_tab(notebook: &Notebook, label: &str, content: &impl IsA<Widget>) {
    notebook.append_page(content, Some(&Label::new(Some(label))));
}

fn draw_axis(area: &DrawingArea, ctx: &gdk4::cairo::Context, _width: i32, _height: i32) {
    let hc = (area.height() as f64) / 2.0;
    let wc = (area.width() as f64) / 2.0;
    ctx.set_source_rgb(0.10, 0.10, 0.10);
    ctx.paint().unwrap();
    ctx.set_source_rgb(1.0, 1.0, 1.0);
    ctx.set_line_width(1.0);
    ctx.move_to(0.0, hc);
    ctx.line_to(area.width().into(), hc);
    ctx.move_to(wc, 0.0);
    ctx.line_to(wc, area.height().into());
    ctx.stroke().unwrap();

    ctx.move_to(wc + 10.0, 15.0);
    ctx.set_font_size(15.0);
    ctx.set_antialias(Antialias::Best);
    ctx.show_text("sigma").unwrap();

    ctx.move_to(area.width() as f64 - 90.0, hc + 15.0);
    ctx.show_text("moneyness").unwrap();
}
