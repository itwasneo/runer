mod custom_button;

use glib::clone;
use gtk::{
    glib, Application, ApplicationWindow, Box, ScrolledWindow, Separator, TextBuffer, TextView,
};
use gtk::{prelude::*, Button};
use gtk4 as gtk;
use std::cell::Cell;
use std::rc::Rc;

use crate::custom_button::CustomButton;

pub fn build_ui(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title("runer")
        .default_height(600)
        .default_width(800)
        .build();

    let scrolled_window = ScrolledWindow::builder()
        .width_request(700)
        .height_request(700)
        .build();

    let button_increase = Button::builder()
        .label("Increase")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let button_decrease = Button::builder()
        .label("Decrease")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let my_button = CustomButton::with_label("Press me!");
    my_button.set_margin_top(12);
    my_button.set_margin_bottom(12);
    my_button.set_margin_start(12);
    my_button.set_margin_end(12);

    let buff = TextBuffer::new(None);
    buff.set_text("This is a text view.\nThis is a text view.");
    let tv = TextView::builder()
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .buffer(&buff)
        .build();

    scrolled_window.set_child(Some(&tv));

    // Reference-countet object with inner-mutability
    let number = Rc::new(Cell::new(0));

    button_increase.connect_clicked(clone!(@weak number, @strong button_decrease => move |_| {
        number.set(number.get() + 1);
        button_decrease.set_label(&number.get().to_string());
    }));
    button_decrease.connect_clicked(clone!(@strong button_increase => move |_| {
        number.set(number.get() - 1);
        button_increase.set_label(&number.get().to_string());
    }));

    my_button.connect_clicked(clone!(@strong buff => move |_| {
        let mut end_iter = buff.end_iter();
        buff.insert(&mut end_iter, "\nNew Message");
    }));

    let gtk_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    gtk_box.append(&button_increase);
    gtk_box.append(&button_decrease);
    gtk_box.append(&Separator::new(gtk::Orientation::Vertical));
    gtk_box.append(&my_button);
    gtk_box.append(&Separator::new(gtk::Orientation::Vertical));
    gtk_box.append(&scrolled_window);

    window.set_child(Some(&gtk_box));

    // Presents the window to the user.
    window.present();
}
