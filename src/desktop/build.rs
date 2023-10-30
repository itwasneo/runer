use super::console::create_console;
use super::plot::draw_axis;
use gdk4::prelude::IsA;
use gtk::{prelude::*, DrawingArea, HeaderBar, Label, Notebook, ScrolledWindow, Widget};
use gtk::{Application, ApplicationWindow, Box};
use gtk4 as gtk;
// use log::{error, info};

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

    create_tab(&gtk_notebook, "Run", &create_run_tab());
    create_tab(&gtk_notebook, "Logs", &console);
    create_tab(&gtk_notebook, "Plot", &da);

    gtk_box.append(&gtk_notebook);

    window.set_child(Some(&gtk_box));

    window.present();
}

fn create_tab(notebook: &Notebook, label: &str, content: &impl IsA<Widget>) {
    notebook.append_page(content, Some(&Label::new(Some(label))));
}

fn create_run_tab() -> Box {
    let box_row = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();

    let scrolled_window = ScrolledWindow::builder()
        .min_content_width(300)
        .has_frame(true)
        .hexpand(false)
        .vexpand(false)
        .build();

    box_row.append(&scrolled_window);

    box_row
}
