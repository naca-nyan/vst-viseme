use nih_plug::{
    params::FloatParam,
    prelude::{FloatRange, ParamSetter},
};
use nih_plug_egui::egui::{
    emath::GuiRounding, pos2, vec2, Color32, Rangef, Rect, Response, Rgba, Sense, Shape, Stroke,
    Ui, Widget,
};

pub struct Meter<'a> {
    value: f32,
    min: f32,
    max: f32,
    floor: &'a FloatParam,
    ceil: &'a FloatParam,
    setter: &'a ParamSetter<'a>,
}

impl<'a> Meter<'a> {
    pub fn new(
        value: f32,
        floor: &'a FloatParam,
        ceil: &'a FloatParam,
        setter: &'a ParamSetter<'a>,
    ) -> Self {
        let (min, max) = if let FloatRange::Linear { min, max } = floor.range() {
            (min, max)
        } else {
            (0.0, 1.0)
        };
        Self {
            value,
            min,
            max,
            floor,
            ceil,
            setter,
        }
    }

    fn normalize(&self, v: f32) -> f32 {
        let v = if v.is_nan() { 0.0 } else { v };
        ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }
}

impl<'a> Widget for Meter<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_width = ui.available_width();
        let interact_height = ui.spacing().interact_size.y;
        let slider_height =
            (interact_height * 0.6).round_to_pixels(ui.painter().pixels_per_point());
        let desired_size = vec2(desired_width, slider_height);
        let (outer_rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        let floor_x = outer_rect.left() + outer_rect.width() * self.normalize(self.floor.value());
        let ceil_x = outer_rect.left() + outer_rect.width() * self.normalize(self.ceil.value());

        if response.clicked() || response.dragged() {
            let x = response.interact_pointer_pos().unwrap().x;
            let ratio = (x - outer_rect.left()) / outer_rect.width();
            let value = self.min + (self.max - self.min) * ratio;
            if (x - floor_x).abs() < (x - ceil_x).abs() {
                self.setter.set_parameter(self.floor, value);
            } else {
                self.setter.set_parameter(self.ceil, value);
            }
        }

        if ui.is_rect_visible(response.rect) {
            let visuals = ui.style().visuals.clone();
            let bg_color = visuals.extreme_bg_color;
            ui.painter().rect_filled(outer_rect, 0, bg_color);

            let filled_height = outer_rect.height() * 0.8;
            let fill_y_range = {
                let center = outer_rect.center().y;
                let size = filled_height;
                Rangef::new(center - size * 0.5, center + size * 0.5)
            };

            let color_light = Color32::from(Rgba::from(visuals.selection.bg_fill) * 0.7);
            let color_dark = Color32::from(Rgba::from(visuals.text_color()) * 0.3);

            let fills = [
                (color_dark, outer_rect.left(), floor_x),
                (color_light, floor_x, ceil_x),
                (color_dark, ceil_x, outer_rect.right()),
            ];
            let value_x = outer_rect.left() + outer_rect.width() * self.normalize(self.value);
            for (fill_color, min, max) in fills {
                let max = max.min(value_x);
                let rect = Rect::from_x_y_ranges(min..=max, fill_y_range);
                ui.painter().rect_filled(rect, 0, fill_color);
            }

            let stroke = Stroke::new(1.0, visuals.text_color());
            for bound_x in [floor_x, ceil_x] {
                ui.painter().vline(bound_x, outer_rect.y_range(), stroke);
                const SIZE: f32 = 3.0;
                let p1 = pos2(bound_x, outer_rect.bottom());
                let p2 = pos2(bound_x + SIZE + 1.0, outer_rect.bottom() + SIZE * 2.0);
                let p3 = pos2(bound_x - SIZE, outer_rect.bottom() + SIZE * 2.0);
                let triangle = Shape::convex_polygon(vec![p1, p2, p3], color_dark, stroke);
                ui.painter().add(triangle);
            }
        }
        response
    }
}
