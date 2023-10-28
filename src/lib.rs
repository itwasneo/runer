mod desktop;

use desktop::logger::init;
use gdk4::glib::{ControlFlow, Priority};
use gtk::{prelude::*, HeaderBar, Label, Notebook, ScrolledWindow, TextBuffer, TextView, Widget};
use gtk::{Application, ApplicationWindow, Box};
use gtk4 as gtk;
use log::{error, info};

pub fn build_ui(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title("runer")
        .default_height(600)
        .default_width(800)
        // .decorated(false)
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

    create_tab(&gtk_notebook, "Logs", &console);
    create_tab(&gtk_notebook, "tab_2", &Label::new(Some("tab_2_content")));
    create_tab(&gtk_notebook, "tab_3", &Label::new(Some("tab_3_content")));

    gtk_box.append(&gtk_notebook);

    window.set_child(Some(&gtk_box));

    window.present();
}

fn create_tab(notebook: &Notebook, label: &str, content: &impl IsA<Widget>) {
    notebook.append_page(content, Some(&Label::new(Some(label))));
}

fn create_console() -> ScrolledWindow {
    let scrollable = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let text_buffer = TextBuffer::new(None);
    let buffer = text_buffer.clone();

    let (tx, rx) = gtk::glib::MainContext::channel::<String>(Priority::DEFAULT);

    init(tx.clone()).unwrap_or_else(|_| error!("Application logger couldn't get initialized"));
    rx.attach(None, move |msg| {
        buffer.insert_at_cursor(&msg);
        ControlFlow::Continue
    });

    info!("Life is beautiful");

    let text_view = TextView::builder()
        .editable(false)
        .wrap_mode(gtk::WrapMode::Word)
        .pixels_below_lines(10)
        .buffer(&text_buffer)
        .margin_start(5)
        .margin_end(5)
        .hexpand(true)
        .vexpand(true)
        .css_classes(vec!["console"])
        .build();

    scrollable.set_child(Some(&text_view));

    scrollable
}
