#![allow(dead_code, deprecated)]

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

pub struct FreqGraph {
    pub area: gtk4::DrawingArea,
    data: Rc<RefCell<Vec<f64>>>,
}

impl FreqGraph {
    pub fn new() -> Self {
        let area = gtk4::DrawingArea::new();
        area.set_content_width(400);
        area.set_content_height(140);
        area.add_css_class("freq-graph");

        let data: Rc<RefCell<Vec<f64>>> = Rc::new(RefCell::new(Vec::new()));
        let data_clone = data.clone();

        area.set_draw_func(move |area, cr, width, height| {
            let data = data_clone.borrow();
            if data.is_empty() || width < 2 || height < 2 {
                draw_empty(area, cr, width, height);
                return;
            }

            let n = data.len();
            let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            let (plot_min, plot_max) = if max_val - min_val < 100.0 {
                let mid = (max_val + min_val) / 2.0;
                ((mid - 50.0).max(0.0), mid + 50.0)
            } else {
                (min_val.max(0.0), max_val)
            };

            let padding_top = 24.0;
            let padding_bottom = 24.0;
            let padding_left = 8.0;
            let padding_right = 8.0;
            let plot_w = width as f64 - padding_left - padding_right;
            let plot_h = height as f64 - padding_top - padding_bottom;

            if plot_w <= 0.0 || plot_h <= 0.0 {
                return;
            }

            let style = area.style_context();
            let fg_color = style
                .lookup_color("accent_bg_color")
                .unwrap_or_else(|| {
                    gtk4::gdk::RGBA::new(0.2078, 0.5176, 0.8941, 1.0)
                });

            let text_color = style
                .lookup_color("window_fg_color")
                .unwrap_or_else(|| {
                    gtk4::gdk::RGBA::new(1.0, 1.0, 1.0, 1.0)
                });

            let to_x = |i: usize| padding_left + (i as f64 / (n - 1).max(1) as f64) * plot_w;
            let to_y = |v: f64| {
                let frac = if plot_max > plot_min {
                    (v - plot_min) / (plot_max - plot_min)
                } else {
                    0.5
                };
                padding_top + plot_h * (1.0 - frac)
            };

            // Gradient fill under line
            cr.move_to(to_x(0), to_y(data[0]));
            for i in 1..n {
                cr.line_to(to_x(i), to_y(data[i]));
            }
            cr.line_to(to_x(n - 1), padding_top + plot_h);
            cr.line_to(to_x(0), padding_top + plot_h);
            cr.close_path();

            let grad = cairo::LinearGradient::new(
                0.0,
                padding_top,
                0.0,
                padding_top + plot_h,
            );
            grad.add_color_stop_rgba(0.0, fg_color.red() as f64, fg_color.green() as f64, fg_color.blue() as f64, 0.25);
            grad.add_color_stop_rgba(1.0, fg_color.red() as f64, fg_color.green() as f64, fg_color.blue() as f64, 0.02);
            cr.set_source(&grad).unwrap();
            cr.fill().unwrap();

            // Line
            cr.set_line_width(2.0);
            cr.set_source_rgba(fg_color.red() as f64, fg_color.green() as f64, fg_color.blue() as f64, 0.9);
            cr.move_to(to_x(0), to_y(data[0]));
            for i in 1..n {
                cr.line_to(to_x(i), to_y(data[i]));
            }
            cr.stroke().unwrap();

            // Labels
            cr.set_source_rgba(text_color.red() as f64, text_color.green() as f64, text_color.blue() as f64, 0.6);
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(10.0);

            let current = data[n - 1];
            let label = format!("{:.0} MHz", current);
            cr.move_to(width as f64 - padding_right - 60.0, padding_top - 6.0);
            cr.show_text(&label).ok();

            cr.move_to(padding_left, padding_top + plot_h + 14.0);
            cr.show_text(&format!("{:.0}", plot_min)).ok();

            cr.move_to(padding_left, padding_top - 6.0);
            cr.show_text(&format!("{:.0}", plot_max)).ok();
        });

        FreqGraph { area, data }
    }

    pub fn set_data(&self, freq_history: Vec<f64>) {
        *self.data.borrow_mut() = freq_history;
        self.area.queue_draw();
    }
}

fn draw_empty(area: &gtk4::DrawingArea, cr: &cairo::Context, width: i32, height: i32) {
    let style = area.style_context();
    let color = style
        .lookup_color("window_fg_color")
        .unwrap_or_else(|| gtk4::gdk::RGBA::new(0.5, 0.5, 0.5, 1.0));

    cr.set_source_rgba(color.red() as f64, color.green() as f64, color.blue() as f64, 0.3);
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(12.0);

    let text = "Waiting for data…";
    if let Ok(extents) = cr.text_extents(text) {
        if width > 0 && height > 0 {
            cr.move_to(
                (width as f64 - extents.width()) / 2.0,
                (height as f64 + extents.height()) / 2.0,
            );
            cr.show_text(text).ok();
        }
    }
}
