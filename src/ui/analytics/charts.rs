use crate::server::analytics_service::{AllowDenyCounts, TimeBucket};

#[cfg(feature = "ssr")]
pub fn render_rps_svg(points: &[TimeBucket]) -> String {
    use plotters::prelude::*;

    let (w, h) = (800u32, 300u32);
    let mut out = String::new();
    {
        let root = SVGBackend::with_string(&mut out, (w, h)).into_drawing_area();
        let _ = root.fill(&WHITE);

        if points.is_empty() {
            let _ = root.draw_text(
                "No data",
                &("sans-serif", 16)
                    .into_text_style(&root)
                    .color(&RGBColor(120, 120, 120)),
                (w as i32 / 2 - 30, h as i32 / 2 - 8),
            );
            let _ = root.present();
        } else {
            let x_min = points.first().map(|p| p.ts_unix_ms).unwrap_or(0);
            let x_max = points.last().map(|p| p.ts_unix_ms).unwrap_or(x_min + 1);
            let y_max = points.iter().map(|p| p.count).max().unwrap_or(1).max(1) as f64;

            let x_range = x_min..x_max.max(x_min + 1);
            let y_range = 0f64..(y_max * 1.1);

            if let Ok(mut chart) = ChartBuilder::on(&root)
                .margin(10)
                .x_label_area_size(30)
                .y_label_area_size(40)
                .build_cartesian_2d(x_range, y_range)
            {
                let _ = chart
                    .configure_mesh()
                    .light_line_style(RGBColor(240, 240, 240))
                    .bold_line_style(RGBColor(220, 220, 220))
                    .axis_style(RGBColor(160, 160, 160))
                    .x_labels(6)
                    .y_labels(5)
                    .label_style(
                        ("sans-serif", 11)
                            .into_font()
                            .color(&RGBColor(100, 100, 100)),
                    )
                    .draw();

                let blue = RGBColor(37, 99, 235);
                let _ = chart.draw_series(LineSeries::new(
                    points.iter().map(|p| (p.ts_unix_ms, p.count as f64)),
                    blue.stroke_width(2),
                ));
            }
            let _ = root.present();
        }
    }
    out
}

#[cfg(feature = "ssr")]
pub fn render_allow_deny_svg(counts: &AllowDenyCounts) -> String {
    use plotters::prelude::*;

    let (w, h) = (600u32, 300u32);
    let mut out = String::new();
    {
        let root = SVGBackend::with_string(&mut out, (w, h)).into_drawing_area();
        let _ = root.fill(&WHITE);

        let total = counts.allowed + counts.denied;
        if total == 0 {
            let _ = root.draw_text(
                "No data",
                &("sans-serif", 16)
                    .into_text_style(&root)
                    .color(&RGBColor(120, 120, 120)),
                (w as i32 / 2 - 30, h as i32 / 2 - 8),
            );
            let _ = root.present();
        } else {
            let y_max = counts.allowed.max(counts.denied) as f64;
            if let Ok(mut chart) = ChartBuilder::on(&root)
                .margin(20)
                .x_label_area_size(40)
                .y_label_area_size(50)
                .build_cartesian_2d(0i32..2i32, 0f64..(y_max * 1.15))
            {
                let _ = chart
                    .configure_mesh()
                    .light_line_style(RGBColor(240, 240, 240))
                    .bold_line_style(RGBColor(220, 220, 220))
                    .axis_style(RGBColor(160, 160, 160))
                    .x_labels(2)
                    .y_labels(5)
                    .x_label_formatter(&|x| match *x {
                        0 => "allowed".to_string(),
                        1 => "denied".to_string(),
                        _ => String::new(),
                    })
                    .label_style(
                        ("sans-serif", 11)
                            .into_font()
                            .color(&RGBColor(100, 100, 100)),
                    )
                    .draw();

                let green = RGBColor(34, 197, 94);
                let red = RGBColor(220, 38, 38);

                let _ = chart.draw_series(std::iter::once(Rectangle::new(
                    [(0i32, 0f64), (0i32, counts.allowed as f64)],
                    green.stroke_width(40),
                )));
                let _ = chart.draw_series(std::iter::once(Rectangle::new(
                    [(1i32, 0f64), (1i32, counts.denied as f64)],
                    red.stroke_width(40),
                )));
            }
            let _ = root.present();
        }
    }
    out
}

#[cfg(not(feature = "ssr"))]
pub fn render_rps_svg(_points: &[TimeBucket]) -> String {
    String::new()
}

#[cfg(not(feature = "ssr"))]
pub fn render_allow_deny_svg(_counts: &AllowDenyCounts) -> String {
    String::new()
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn render_rps_svg_empty_returns_svg() {
        let s = render_rps_svg(&[]);
        assert!(s.starts_with("<svg"), "got: {}", &s[..s.len().min(60)]);
    }

    #[test]
    fn render_rps_svg_with_points_returns_svg() {
        let pts = vec![
            TimeBucket {
                ts_unix_ms: 1_700_000_000_000,
                count: 10,
            },
            TimeBucket {
                ts_unix_ms: 1_700_000_060_000,
                count: 25,
            },
            TimeBucket {
                ts_unix_ms: 1_700_000_120_000,
                count: 5,
            },
        ];
        let s = render_rps_svg(&pts);
        assert!(s.starts_with("<svg"));
    }

    #[test]
    fn render_allow_deny_svg_zero_returns_svg() {
        let s = render_allow_deny_svg(&AllowDenyCounts {
            allowed: 0,
            denied: 0,
        });
        assert!(s.starts_with("<svg"));
    }

    #[test]
    fn render_allow_deny_svg_with_counts_returns_svg() {
        let s = render_allow_deny_svg(&AllowDenyCounts {
            allowed: 120,
            denied: 7,
        });
        assert!(s.starts_with("<svg"));
    }
}
