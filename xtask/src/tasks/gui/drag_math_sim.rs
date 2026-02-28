use anyhow::Result;
use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct DragMathSimArgs {
    #[arg(long, default_value_t = 24)]
    pub base_left: i32,
    #[arg(long, default_value_t = 24)]
    pub base_top: i32,
    #[arg(long, default_value_t = 1)]
    pub scale_factor: i32,
    #[arg(long, default_value_t = 240.0)]
    pub total_dx: f64,
    #[arg(long, default_value_t = 0.0)]
    pub total_dy: f64,
    #[arg(long, default_value_t = 12)]
    pub steps: usize,
    #[arg(long, default_value_t = 3000)]
    pub max_left: i32,
    #[arg(long, default_value_t = 3000)]
    pub max_top: i32,
}

pub fn run(args: DragMathSimArgs) -> Result<()> {
    let steps = args.steps.max(1);
    let scale_factor = args.scale_factor.max(1);

    println!(
        "Drag math simulation\nbase=({}, {}) scale_factor={} bounds=({}, {}) total_delta=({:.3}, {:.3}) steps={}",
        args.base_left,
        args.base_top,
        scale_factor,
        args.max_left,
        args.max_top,
        args.total_dx,
        args.total_dy,
        steps
    );
    println!(
        "{:>4} | {:>9} {:>9} | {:>8} {:>8} | {:>8} {:>8}",
        "step", "dx", "dy", "logical_x", "logical_y", "scaled_x", "scaled_y"
    );

    let mut logical_last = (args.base_left, args.base_top);
    let mut scaled_last = (args.base_left, args.base_top);

    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let dx = args.total_dx * t;
        let dy = args.total_dy * t;

        logical_last = position_for(
            args.base_left,
            args.base_top,
            dx,
            dy,
            1,
            args.max_left,
            args.max_top,
        );
        scaled_last = position_for(
            args.base_left,
            args.base_top,
            dx,
            dy,
            scale_factor,
            args.max_left,
            args.max_top,
        );

        println!(
            "{:>4} | {:>9.3} {:>9.3} | {:>8} {:>8} | {:>8} {:>8}",
            step, dx, dy, logical_last.0, logical_last.1, scaled_last.0, scaled_last.1
        );
    }

    let pointer_distance_x = args.total_dx.abs();
    let pointer_distance_y = args.total_dy.abs();
    let logical_ratio_x = speed_ratio(args.base_left, logical_last.0, pointer_distance_x);
    let scaled_ratio_x = speed_ratio(args.base_left, scaled_last.0, pointer_distance_x);
    let logical_ratio_y = speed_ratio(args.base_top, logical_last.1, pointer_distance_y);
    let scaled_ratio_y = speed_ratio(args.base_top, scaled_last.1, pointer_distance_y);

    println!();
    println!(
        "speed ratio (window_delta / pointer_delta): logical=({:.3}, {:.3}) scaled=({:.3}, {:.3})",
        logical_ratio_x, logical_ratio_y, scaled_ratio_x, scaled_ratio_y
    );
    println!(
        "Use this to match observed behavior: if real drag is ~0.5x at sf=2, scaled math is likely required; if real drag is ~2x, logical math is required."
    );

    Ok(())
}

fn position_for(
    base_left: i32,
    base_top: i32,
    offset_x: f64,
    offset_y: f64,
    scale_factor: i32,
    max_left: i32,
    max_top: i32,
) -> (i32, i32) {
    let scale = scale_factor.max(1) as f64;
    let left = ((base_left as f64) + (offset_x * scale)).round() as i32;
    let top = ((base_top as f64) + (offset_y * scale)).round() as i32;
    (left.clamp(0, max_left), top.clamp(0, max_top))
}

fn speed_ratio(base: i32, current: i32, pointer_delta: f64) -> f64 {
    if pointer_delta <= f64::EPSILON {
        return 0.0;
    }

    (current - base).abs() as f64 / pointer_delta
}
