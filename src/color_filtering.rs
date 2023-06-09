pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let f_r = r as f32 / 255.0;
    let f_g = g as f32 / 255.0;
    let f_b = b as f32 / 255.0;

    let f_c_max = f_r.max(f_g).max(f_b);
    let f_c_min = f_r.min(f_g).min(f_b);
    let f_delta = f_c_max - f_c_min;

    let mut f_h = 0.0;
    let mut f_s = 0.0;
    let f_v = f_c_max;

    if f_delta > 0.0 {
        if f_c_max == f_r {
            f_h = 60.0 * ((f_g - f_b) / f_delta).rem_euclid(6.0);
        } else if f_c_max == f_g {
            f_h = 60.0 * ((f_b - f_r) / f_delta + 2.0);
        } else if f_c_max == f_b {
            f_h = 60.0 * ((f_r - f_g) / f_delta + 4.0);
        }

        if f_c_max > 0.0 {
            f_s = f_delta / f_c_max;
        }
    }

    (f_h, f_s, f_v)
}

pub fn threshold_white(image_data: &[u8], target_color: (f32, f32, f32)) -> Vec<u8> {
    let mut new_image_data = Vec::with_capacity(image_data.len() / 4);
    let mut index = 0;

    while index + 4 <= image_data.len() {
        let (h, s, v) = rgb_to_hsv(image_data[index+2], image_data[index+1], image_data[index]);
        let similarity = calculate_hsv_similarity(target_color, (h,s,v), 0.0);
        new_image_data.push((similarity * 255.0) as u8);

        index += 4;
    }

    new_image_data
}

pub fn calculate_hsv_similarity(hsv1: (f32, f32, f32), hsv2: (f32, f32, f32), max_h_diff: f32) -> f32 {
    let (h1, s1, v1) = hsv1;
    let (h2, s2, v2) = hsv2;

    let dh = (h1 - h2).abs();
    let ds = (s1 - s2).abs();
    let dv = (v1 - v2).abs();

    // Adjust the hue difference for cyclic nature of hue values
    let hue_difference = if dh > 180.0 { 360.0 - dh } else { dh };

    if hue_difference <= max_h_diff {
        // Calculate the Euclidean distance between HSV values
        let distance = (hue_difference.powi(2) + ds.powi(2) + dv.powi(2)).sqrt();
        return 1.0 - (distance / (3.0 * 100.0_f32.sqrt()));
    }

    0.0
}

pub fn find_head_point(image: &[u8]) -> Option<(usize, usize)> {
    let image_size = (image.len() as f64).sqrt() as usize;
    
    // Find highest_y
    let highest_y = (0..image_size/2)
        .rev()
        .find(|&row| image[row * image_size + 50] > 0)?;

    // Scan left to find a value > 0
    let left_point = (0..image_size/2)
        .rev()
        .find(|&col| image[(highest_y + 1) * image_size + col] > 0)?;

    // Scan right to find a value > 0
    let right_point = (image_size/2+1..image_size)
        .find(|&col| image[(highest_y + 1) * image_size + col] > 0)?;

    // Calculate the middle x coordinate
    let middle_x = (left_point + right_point) / 2;

    // Return the result
    Some((middle_x, highest_y + 1))
}
