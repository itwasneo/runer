use gdk4::glib::{ControlFlow, Priority};
use gtk::traits::TextBufferExt;
use gtk::{ScrolledWindow, TextBuffer, TextView};
use gtk4 as gtk;
use log::{error, info};

use crate::desktop::logger::init;

pub fn create_console() -> ScrolledWindow {
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
        buffer.insert_at_cursor("\n");
        ControlFlow::Continue
    });

    info!("Logger is indeed attached.");

    let text_view = TextView::builder()
        .editable(false)
        .wrap_mode(gtk::WrapMode::Word)
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
