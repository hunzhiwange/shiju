use super::*;

pub fn zoom_by(pan: Vector, zoom: f32, factor: f32, center_opt: Option<Point>, window_size: (f32, f32)) -> (Vector, f32) {
    let old_zoom = zoom.max(0.0001);
    let new_zoom = (old_zoom * factor).clamp(0.1, 4.0);
    let center = center_opt.unwrap_or(Point::new(window_size.0 / 2.0, window_size.1 / 2.0));
    let center_vector = Vector::new(center.x, center.y);

    let pan = center_vector - (center_vector - pan) * (new_zoom / old_zoom);
    let zoom = new_zoom;
    (pan, zoom)
}

pub fn zoom_to_fit(document: &WorkflowDocument, window_size: (f32, f32)) -> Option<(Vector, f32)> {
    let Some(bounds) = document.bounds() else {
        return None;
    };

    let usable_width = (window_size.0 - 72.0).max(320.0);
    let usable_height = (window_size.1 - 176.0).max(260.0);
    let scale_x = usable_width / bounds.width.max(1.0);
    let scale_y = usable_height / bounds.height.max(1.0);
    let zoom = (scale_x.min(scale_y) * 0.92).clamp(0.1, 4.0);

    let world_center =
        Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0);
    let screen_center = Vector::new(usable_width / 2.0 + 24.0, usable_height / 2.0 + 24.0);

    Some((screen_center - Vector::new(world_center.x * zoom, world_center.y * zoom), zoom))
}
