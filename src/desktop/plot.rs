use gdk4::cairo::Antialias;
use gtk4::prelude::WidgetExt;
use gtk4::DrawingArea;

pub fn draw_axis(area: &DrawingArea, ctx: &gdk4::cairo::Context, _width: i32, _height: i32) {
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
