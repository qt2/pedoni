use glam::Vec2;

pub fn poisson(lambda: f64) -> i32 {
    let mut y = 0;
    let mut x = fastrand::f64();
    let exp_lambda = (-lambda).exp();

    while x >= exp_lambda {
        x *= fastrand::f64();
        y += 1;
    }

    y
}

pub fn distance_from_line(point: Vec2, line: [Vec2; 2]) -> f32 {
    let a = point - line[0];
    let b = line[1] - line[0];
    let b_len2 = b.length_squared();

    if b_len2 == 0.0 {
        (a - line[0]).length()
    } else {
        let t = (a.dot(b) / b_len2).max(0.0).min(1.0);
        (t * b - a).length()
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use glam::vec2;

    use super::distance_from_line;

    #[test]
    fn test_distance_from_line() {
        let line = [vec2(1.0, 1.0), vec2(4.0, 1.0)];

        assert_float_absolute_eq!(distance_from_line(vec2(2.0, 3.0), line), 2.0);
        assert_float_absolute_eq!(distance_from_line(vec2(0.0, 0.25), line), 1.25);
    }
}
