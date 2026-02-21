use flutter_rust_bridge::frb;

use super::state::with_state;

/// Measurement type
#[derive(Debug, Clone)]
pub enum MeasurementType {
    Distance,
    Area,
    Volume,
}

/// Measurement point in 3D space
#[derive(Debug, Clone)]
pub struct MeasurementPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Measurement result
#[derive(Debug, Clone)]
pub struct MeasurementResult {
    pub measurement_type: String,
    pub value: f64,
    pub unit: String,
    pub points: Vec<MeasurementPoint>,
}

/// Start a new measurement
#[frb(sync)]
pub fn start_measurement(measurement_type: String) -> Result<(), String> {
    with_state(|s| {
        s.measurement_points.clear();
        s.measurement_type = Some(match measurement_type.as_str() {
            "distance" => MeasurementType::Distance,
            "area" => MeasurementType::Area,
            "volume" => MeasurementType::Volume,
            _ => return Err(format!("Invalid measurement type: {}", measurement_type)),
        });
        Ok(())
    })
}

/// Add a measurement point
#[frb(sync)]
pub fn add_measurement_point(x: f32, y: f32, z: f32) -> Result<i32, String> {
    with_state(|s| {
        s.measurement_points.push(MeasurementPoint { x, y, z });
        Ok(s.measurement_points.len() as i32)
    })
}

/// Get the current measurement result
#[frb(sync)]
pub fn get_measurement_result() -> Result<MeasurementResult, String> {
    with_state(|s| {
        let measurement_type = s
            .measurement_type
            .as_ref()
            .ok_or("No measurement in progress")?;
        let points = &s.measurement_points;

        match measurement_type {
            MeasurementType::Distance => {
                if points.len() < 2 {
                    return Err(
                        "Need at least 2 points for distance measurement".to_string()
                    );
                }

                let mut total_distance = 0.0;
                for i in 0..points.len() - 1 {
                    let p1 = &points[i];
                    let p2 = &points[i + 1];
                    let dx = p2.x - p1.x;
                    let dy = p2.y - p1.y;
                    let dz = p2.z - p1.z;
                    total_distance += ((dx * dx + dy * dy + dz * dz) as f64).sqrt();
                }

                Ok(MeasurementResult {
                    measurement_type: "distance".to_string(),
                    value: total_distance,
                    unit: "m".to_string(),
                    points: points.clone(),
                })
            }
            MeasurementType::Area => {
                if points.len() < 3 {
                    return Err("Need at least 3 points for area measurement".to_string());
                }

                let mut area = 0.0;
                for i in 0..points.len() {
                    let j = (i + 1) % points.len();
                    area +=
                        (points[i].x * points[j].y - points[j].x * points[i].y) as f64;
                }
                area = (area / 2.0).abs();

                Ok(MeasurementResult {
                    measurement_type: "area".to_string(),
                    value: area,
                    unit: "m²".to_string(),
                    points: points.clone(),
                })
            }
            MeasurementType::Volume => {
                if points.len() < 4 {
                    return Err(
                        "Need at least 4 points for volume measurement".to_string()
                    );
                }

                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                let mut min_z = f32::MAX;
                let mut max_z = f32::MIN;

                for p in points.iter() {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                    min_z = min_z.min(p.z);
                    max_z = max_z.max(p.z);
                }

                let width = (max_x - min_x) as f64;
                let depth = (max_y - min_y) as f64;
                let height = (max_z - min_z) as f64;
                let volume = width * depth * height;

                Ok(MeasurementResult {
                    measurement_type: "volume".to_string(),
                    value: volume,
                    unit: "m³".to_string(),
                    points: points.clone(),
                })
            }
        }
    })
}

/// Clear the current measurement
#[frb(sync)]
pub fn clear_measurement() {
    with_state(|s| {
        s.measurement_points.clear();
        s.measurement_type = None;
    });
}

/// Get the number of measurement points
#[frb(sync)]
pub fn get_measurement_point_count() -> i32 {
    with_state(|s| s.measurement_points.len() as i32)
}
