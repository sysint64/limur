use std::time::Duration;

use crate::{ColorOkLab, ColorRgb, ColorRgba, EdgeInsets, Value};

#[derive(Debug, Clone)]
pub struct Tween<V> {
    t: f64,
    start_value: V,
    current_value: V,
    target_value: V,
    status: AnimationStatus,
    duration: f64,
    curve_fn: fn(t: f64) -> f64,
    repeat: Repeat,
    cycles_done: u32,
    reverse: bool,
}

#[derive(Debug, Clone)]
pub struct Damp<V> {
    speed: f64,
    current_value: V,
    target_value: V,
    status: AnimationStatus,
    threshold: f64,
    curve_fn: fn(t: f64) -> f64,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AnimationStatus {
    Idle,
    Started,
    Updated,
    Ended,
}

pub trait Animation {
    fn step(&mut self, delta_time: f64);

    fn in_progress(&self) -> bool;
}

pub trait Lerp {
    fn lerp(self, to: Self, t: f64) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, to: f32, t: f64) -> Self {
        ((self as f64 * (1.0 - t)) + (to as f64 * t)) as f32
    }
}

impl Lerp for f64 {
    fn lerp(self, to: f64, t: f64) -> Self {
        (self * (1.0 - t)) + (to * t)
    }
}

impl Lerp for EdgeInsets {
    fn lerp(self, to: Self, t: f64) -> Self {
        EdgeInsets {
            top: f64::lerp(self.top, to.top, t),
            left: f64::lerp(self.left, to.left, t),
            right: f64::lerp(self.right, to.right, t),
            bottom: f64::lerp(self.bottom, to.bottom, t),
        }
    }
}

impl Lerp for ColorOkLab {
    fn lerp(self, to: Self, t: f64) -> Self {
        ColorOkLab {
            l: f64::lerp(self.l, to.l, t),
            a: f64::lerp(self.a, to.a, t),
            b: f64::lerp(self.b, to.b, t),
        }
    }
}

impl Lerp for ColorRgb {
    fn lerp(self, to: Self, t: f64) -> Self {
        if t == 0. {
            return self;
        }

        if t == 1. {
            return to;
        }

        let oklab_self = self.to_oklab();
        let oklab_to = to.to_oklab();
        let interpolated = oklab_self.lerp(oklab_to, t);

        interpolated.to_rgb()
    }
}

impl Lerp for ColorRgba {
    fn lerp(self, to: Self, t: f64) -> Self {
        if t == 0. {
            return self;
        }

        if t == 1. {
            return to;
        }

        let interpolated_rgb = self.rgb().lerp(to.rgb(), t);

        ColorRgba {
            r: interpolated_rgb.r,
            g: interpolated_rgb.g,
            b: interpolated_rgb.b,
            a: f32::lerp(self.a, to.a, t),
        }
    }
}

impl<V> Default for Tween<V>
where
    V: Default,
{
    fn default() -> Self {
        Self {
            t: 1.,
            start_value: V::default(),
            current_value: V::default(),
            target_value: V::default(),
            status: AnimationStatus::Idle,
            curve_fn: curves::ease_out_quad,
            duration: Duration::from_millis(300).as_secs_f64(),
            repeat: Repeat::Once,
            cycles_done: 0,
            reverse: false,
        }
    }
}

impl<V> Tween<V>
where
    V: Lerp + Clone,
{
    pub fn new(value: V) -> Self {
        Self {
            t: 1.,
            start_value: value.clone(),
            current_value: value.clone(),
            target_value: value,
            status: AnimationStatus::Idle,
            curve_fn: curves::ease_out_quad,
            duration: Duration::from_millis(300).as_secs_f64(),
            repeat: Repeat::Once,
            cycles_done: 0,
            reverse: false,
        }
    }

    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;

        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration.as_secs_f64();

        self
    }

    pub fn curve(mut self, curve_fn: fn(t: f64) -> f64) -> Self {
        self.curve_fn = curve_fn;

        self
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
        self.cycles_done = 0;
        self.reverse = false;
        self.status = AnimationStatus::Started;
    }

    pub fn status(&self) -> AnimationStatus {
        self.status
    }

    pub fn tween_to(&mut self, target: V) {
        self.t = 0.0;
        self.cycles_done = 0;
        self.reverse = false;

        self.start_value = self.current_value.clone();
        self.target_value = target;
        self.status = AnimationStatus::Started;
    }

    pub fn set(&mut self, target: V) {
        self.t = 1.;
        self.target_value = target;
        self.start_value = self.target_value.clone();
        self.current_value = self.target_value.clone();

        if self.status == AnimationStatus::Updated {
            self.status = AnimationStatus::Ended;
        } else {
            self.status = AnimationStatus::Idle;
        }
    }

    pub fn t(&self) -> f64 {
        self.t
    }

    fn should_continue(&self) -> bool {
        match self.repeat {
            Repeat::Once => self.cycles_done < 1,
            Repeat::Loop => true,
            Repeat::LoopNCycles(n) => self.cycles_done < n,
            Repeat::PingPong => true,
            Repeat::PingPongNCycles(n) => self.cycles_done < n,
        }
    }
}

impl<V> Value<V> for Tween<V>
where
    V: Clone,
{
    fn value(&self) -> V {
        self.current_value.clone()
    }
}

impl<V> Animation for Tween<V>
where
    V: Lerp + Clone,
{
    fn step(&mut self, delta_time: f64) {
        if self.status == AnimationStatus::Ended {
            self.status = AnimationStatus::Idle;
            return;
        }

        if self.status == AnimationStatus::Idle {
            return;
        }

        self.t += delta_time / self.duration.max(0.000_001);

        if self.t < 1.0 {
            self.status = AnimationStatus::Updated;
            let mut t = self.t;

            if self.reverse {
                t = 1.0 - t;
            }

            self.current_value = V::lerp(
                self.start_value.clone(),
                self.target_value.clone(),
                (self.curve_fn)(t),
            );

            return;
        }

        // Clamp end of segment
        self.t = 1.0;

        // Final value of this cycle
        self.current_value = if self.reverse {
            self.start_value.clone()
        } else {
            self.target_value.clone()
        };

        self.cycles_done += 1;

        if !self.should_continue() {
            self.status = AnimationStatus::Ended;
            return;
        }

        // Prepare next cycle
        self.t = 0.0;

        match self.repeat {
            Repeat::Once => {
                self.status = AnimationStatus::Ended;
            }
            Repeat::Loop | Repeat::LoopNCycles(_) => {
                // restart from original direction
                self.reverse = false;
                self.status = AnimationStatus::Updated;
            }
            Repeat::PingPong | Repeat::PingPongNCycles(_) => {
                // swap direction
                self.reverse = !self.reverse;
                self.status = AnimationStatus::Updated;
            }
        }
    }

    fn in_progress(&self) -> bool {
        self.status != AnimationStatus::Idle
    }
}

impl<V> Damp<V>
where
    V: Lerp + Clone,
{
    pub fn new(value: V) -> Self {
        Self {
            speed: 10.,
            current_value: value.clone(),
            target_value: value,
            curve_fn: decay_curves::default,
            status: AnimationStatus::Idle,
            threshold: 0.01,
        }
    }

    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn curve(mut self, curve_fn: fn(t: f64) -> f64) -> Self {
        self.curve_fn = curve_fn;
        self
    }

    pub fn threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn set(&mut self, value: V) {
        self.current_value = value.clone();
        self.target_value = value;

        if self.status == AnimationStatus::Updated {
            self.status = AnimationStatus::Ended;
        } else {
            self.status = AnimationStatus::Idle;
        }
    }

    pub fn status(&self) -> AnimationStatus {
        self.status
    }
}

impl<V> Damp<V>
where
    V: Lerp + Clone + Difference,
{
    pub fn approach(&mut self, target: V) {
        if self.current_value.difference(&target) > self.threshold {
            self.target_value = target;

            if self.status != AnimationStatus::Updated {
                self.status = AnimationStatus::Started;
            }
        } else {
            // Already close, snap to target and stop
            self.target_value = target.clone();
            self.current_value = target;

            if self.status == AnimationStatus::Updated || self.status == AnimationStatus::Started {
                self.status = AnimationStatus::Ended;
            } else {
                self.status = AnimationStatus::Idle;
            }
        }
    }
}

impl<V> Value<V> for Damp<V>
where
    V: Clone,
{
    fn value(&self) -> V {
        self.current_value.clone()
    }
}

impl<V> Animation for Damp<V>
where
    V: Lerp + Clone + Difference,
{
    fn step(&mut self, delta_time: f64) {
        if self.status == AnimationStatus::Ended {
            self.status = AnimationStatus::Idle;
            return;
        }

        if self.status == AnimationStatus::Idle {
            return;
        }

        let distance = self.current_value.difference(&self.target_value);

        if distance < self.threshold {
            self.current_value = self.target_value.clone();
            self.status = AnimationStatus::Ended;
        } else {
            let t = (self.curve_fn)(self.speed * delta_time);
            self.current_value = V::lerp(self.current_value.clone(), self.target_value.clone(), t);
            self.status = AnimationStatus::Updated;
        }
    }

    fn in_progress(&self) -> bool {
        self.status != AnimationStatus::Idle
    }
}

pub trait Difference {
    fn difference(&self, other: &Self) -> f64;
}

impl Difference for f32 {
    fn difference(&self, other: &Self) -> f64 {
        (self - other).abs() as f64
    }
}

impl Difference for f64 {
    fn difference(&self, other: &Self) -> f64 {
        (self - other).abs()
    }
}

impl Difference for EdgeInsets {
    fn difference(&self, other: &Self) -> f64 {
        (self.top - other.top).abs()
            + (self.left - other.left).abs()
            + (self.right - other.right).abs()
            + (self.bottom - other.bottom).abs()
    }
}

impl Difference for ColorOkLab {
    fn difference(&self, other: &Self) -> f64 {
        (self.l - other.l).abs() + (self.a - other.a).abs() + (self.b - other.b).abs()
    }
}

pub mod curves {
    // Linear
    pub fn linear(t: f64) -> f64 {
        t
    }

    pub fn smooth_step(t: f64) -> f64 {
        t * t * (3. - 2. * t)
    }

    pub fn smoother_step(t: f64) -> f64 {
        t * t * t * (t * (6. * t - 15.) + 10.)
    }

    // Quadratic
    pub fn ease_in_quad(t: f64) -> f64 {
        t * t
    }

    pub fn ease_out_quad(t: f64) -> f64 {
        1. - (1. - t) * (1. - t)
    }

    pub fn ease_in_out_quad(t: f64) -> f64 {
        if t < 0.5 {
            2. * t * t
        } else {
            1. - (-2. * t + 2.).powi(2) / 2.
        }
    }

    // Cubic
    pub fn ease_in_cubic(t: f64) -> f64 {
        t * t * t
    }

    pub fn ease_out_cubic(t: f64) -> f64 {
        1. - (1. - t).powi(3)
    }

    pub fn ease_in_out_cubic(t: f64) -> f64 {
        if t < 0.5 {
            4. * t * t * t
        } else {
            1. - (-2. * t + 2.).powi(3) / 2.
        }
    }

    // Sine
    pub fn ease_in_sine(t: f64) -> f64 {
        1. - f64::cos(t * std::f64::consts::FRAC_PI_2)
    }

    pub fn ease_out_sine(t: f64) -> f64 {
        f64::sin(t * std::f64::consts::FRAC_PI_2)
    }

    pub fn ease_in_out_sine(t: f64) -> f64 {
        -(f64::cos(std::f64::consts::PI * t) - 1.) / 2.
    }

    // Exponential
    pub fn ease_in_expo(t: f64) -> f64 {
        if t == 0. {
            0.
        } else {
            f64::powf(2., 10. * t - 10.)
        }
    }

    pub fn ease_out_expo(t: f64) -> f64 {
        if t == 1. {
            1.
        } else {
            1. - f64::powf(2., -10. * t)
        }
    }

    // Back (overshoot)
    pub fn ease_in_back(t: f64) -> f64 {
        let c1 = 1.70158;
        let c3 = c1 + 1.;
        c3 * t * t * t - c1 * t * t
    }

    pub fn ease_out_back(t: f64) -> f64 {
        let c1 = 1.70158;
        let c3 = c1 + 1.;
        1. + c3 * (t - 1.).powi(3) + c1 * (t - 1.).powi(2)
    }

    // Elastic
    pub fn ease_out_elastic(t: f64) -> f64 {
        if t == 0. {
            0.
        } else if t == 1. {
            1.
        } else {
            let c4 = (2. * std::f64::consts::PI) / 3.;
            f64::powf(2., -10. * t) * f64::sin((t * 10. - 0.75) * c4) + 1.
        }
    }

    // Bounce
    pub fn ease_out_bounce(t: f64) -> f64 {
        let n1 = 7.5625;
        let d1 = 2.75;

        if t < 1. / d1 {
            n1 * t * t
        } else if t < 2. / d1 {
            let t = t - 1.5 / d1;
            n1 * t * t + 0.75
        } else if t < 2.5 / d1 {
            let t = t - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }
}

pub mod decay_curves {
    // Slower decay
    pub fn slow(t: f64) -> f64 {
        1. - f64::powf(0.7, t)
    }

    // Default
    pub fn default(t: f64) -> f64 {
        1. - f64::powf(0.5, t)
    }

    // Faster decay
    pub fn fast(t: f64) -> f64 {
        1. - f64::powf(0.2, t)
    }

    // Very snappy
    pub fn snappy(t: f64) -> f64 {
        1. - f64::powf(0.05, t)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Repeat {
    /// Play the animation once.
    Once,

    /// Repeat the animation indefinitely, restarting from the beginning each cycle.
    Loop,

    /// Repeat the animation `n` times.
    ///
    /// `n` counts **completed cycles** (one full run from start to end).
    /// For example, `LoopNCycles(1)` is equivalent to `Once`.
    LoopNCycles(u32),

    /// Repeat the animation indefinitely, alternating direction each cycle:
    /// forward, backward, forward, backward, ...
    PingPong,

    /// Repeat the animation `n` times, alternating direction each cycle:
    /// forward, backward, forward, backward, ...
    ///
    /// `n` counts **cycles**, where one cycle is a full traversal from start to end
    /// in the current direction.
    ///
    /// Examples (for a start→end animation):
    /// - `PingPongNCycles(1)`: forward only (start -> end)
    /// - `PingPongNCycles(2)`: forward then backward (start -> end -> start)
    /// - `PingPongNCycles(3)`: forward, backward, forward (start -> end -> start -> end)
    ///
    /// Note: If you want "there and back" to count as 1, use `PingPongNCycles(2)`
    /// with this interpretation, or adjust the implementation to count periods
    /// instead of cycles.
    PingPongNCycles(u32),
}

#[derive(Debug, Clone, Copy)]
pub enum FrameKind {
    /// Stay at the previous value for `dt`, then jump to this keyframe's value at the end.
    Hold,
    /// Interpolate from previous value to this value over `duration`.
    /// If `curve` is None, uses the Keyframes' default curve.
    Tween { curve: Option<fn(f64) -> f64> },
}

#[derive(Debug, Clone, Copy)]
pub struct Keyframe<V> {
    /// Duration of the segment from previous keyframe -> this keyframe.
    /// (For the very first frame, this should be 0.)
    pub duration: Duration,
    pub value: V,
    pub kind: FrameKind,
}

#[derive(Debug, Clone)]
pub struct Keyframes<V> {
    frames: Vec<Keyframe<V>>,
    /// Seconds from start, within the current cycle (0..=total_cycle_seconds)
    elapsed: f64,
    /// Cached total duration of one forward cycle
    total: f64,
    /// Default curve used for Tween segments when keyframe curve is None
    default_curve: fn(f64) -> f64,

    repeat: Repeat,
    cycles_done: u32,
    pingpong_forward: bool,

    current_value: V,
    status: AnimationStatus,
}

impl<V> Keyframes<V>
where
    V: Clone,
{
    /// Creates an animation with a single initial keyframe (dt=0).
    pub fn new(initial: V) -> Self {
        let frames = vec![Keyframe {
            duration: Duration::from_millis(0),
            value: initial.clone(),
            kind: FrameKind::Hold, // irrelevant for the first frame
        }];

        Self {
            frames,
            elapsed: 0.0,
            total: 0.0,
            default_curve: curves::ease_out_quad,

            repeat: Repeat::Once,
            cycles_done: 0,
            pingpong_forward: true,

            current_value: initial,
            status: AnimationStatus::Idle,
        }
    }

    /// Set the default curve used for Tween segments when a keyframe doesn't specify a curve.
    pub fn default_curve(mut self, curve: fn(f64) -> f64) -> Self {
        self.default_curve = curve;

        self
    }

    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;

        self
    }

    pub fn tween(mut self, duration: Duration, value: V) -> Self {
        self.frames.push(Keyframe {
            duration,
            value,
            kind: FrameKind::Tween { curve: None },
        });

        self
    }

    pub fn tween_with_curve(mut self, duration: Duration, value: V, curve: fn(f64) -> f64) -> Self {
        self.frames.push(Keyframe {
            duration,
            value,
            kind: FrameKind::Tween { curve: Some(curve) },
        });
        self
    }

    /// Append a segment that holds the previous value for `duration`, then snaps to `value` at the end.
    pub fn hold(mut self, duration: Duration, value: V) -> Self {
        self.frames.push(Keyframe {
            duration,
            value,
            kind: FrameKind::Hold,
        });
        self
    }

    /// Start/restart playback from the beginning of the cycle.
    pub fn play(&mut self) {
        self.recompute_total();
        self.elapsed = 0.0;
        self.cycles_done = 0;
        self.pingpong_forward = true;

        self.current_value = self.frames.first().unwrap().value.clone();
        self.status = AnimationStatus::Started;
    }

    /// Immediately set the animation to a constant value and stop.
    pub fn set(&mut self, value: V) {
        self.frames.clear();
        self.frames.push(Keyframe {
            duration: Duration::from_millis(0),
            value: value.clone(),
            kind: FrameKind::Hold,
        });

        self.elapsed = 0.0;
        self.total = 0.0;
        self.current_value = value;

        if self.status == AnimationStatus::Updated {
            self.status = AnimationStatus::Ended;
        } else {
            self.status = AnimationStatus::Idle;
        }
    }

    pub fn status(&self) -> AnimationStatus {
        self.status
    }

    pub fn in_progress(&self) -> bool {
        self.status != AnimationStatus::Idle
    }

    fn recompute_total(&mut self) {
        // sum surations of frames[1..]
        self.total = self
            .frames
            .iter()
            .skip(1)
            .map(|k| k.duration.as_secs_f64())
            .sum();

        // avoid division-by-zero; if total==0, we'll just snap to last frame
        if self.total <= 0.0 {
            self.total = 0.0;
        }
    }

    fn should_continue(&self) -> bool {
        match self.repeat {
            Repeat::Once => self.cycles_done < 1,
            Repeat::Loop => true,
            Repeat::LoopNCycles(n) => self.cycles_done < n,
            Repeat::PingPong => true,
            Repeat::PingPongNCycles(n) => self.cycles_done < n,
        }
    }

    fn on_cycle_end(&mut self) {
        self.cycles_done = self.cycles_done.saturating_add(1);

        if !self.should_continue() {
            self.status = AnimationStatus::Ended;
            return;
        }

        match self.repeat {
            Repeat::Once => {
                // If should_continue() is correct, we should never land here.
                self.status = AnimationStatus::Ended;
            }
            Repeat::Loop | Repeat::LoopNCycles(_) => {
                self.elapsed = 0.0;
                self.status = AnimationStatus::Updated;
            }
            Repeat::PingPong | Repeat::PingPongNCycles(_) => {
                self.elapsed = 0.0;
                self.pingpong_forward = !self.pingpong_forward;
                self.status = AnimationStatus::Updated;
            }
        }
    }

    /// Evaluate the value at local time `t` in [0..=total] for the "forward" direction.
    fn eval_forward(&self, mut t: f64) -> V
    where
        V: Lerp,
    {
        let n = self.frames.len();

        if n == 0 {
            // shouldn't happen, but be safe
            return self.current_value.clone();
        }

        if n == 1 || self.total <= 0.0 {
            return self.frames.last().unwrap().value.clone();
        }

        // Clamp within cycle
        t = t.clamp(0.0, self.total);

        // Find segment with a linear scan (fine for <=10 frames)
        let mut seg_start_time = 0.0;

        for i in 1..n {
            let seg = self.frames[i].clone();
            let seg_len = seg.duration.as_secs_f64().max(0.0);
            let seg_end_time = seg_start_time + seg_len;

            if t <= seg_end_time || i == n - 1 {
                let from = self.frames[i - 1].value.clone();
                let to = seg.value.clone();

                // zero-length segment: snap to end
                if seg_len <= 0.0 {
                    return to;
                }

                match seg.kind {
                    FrameKind::Hold => {
                        // hold "from" until end, then snap to "to"
                        if t < seg_end_time {
                            return from;
                        } else {
                            return to;
                        }
                    }
                    FrameKind::Tween { curve } => {
                        let local = ((t - seg_start_time) / seg_len).clamp(0.0, 1.0);
                        let curve_fn = curve.unwrap_or(self.default_curve);
                        let eased = curve_fn(local);

                        return V::lerp(from, to, eased);
                    }
                }
            }

            seg_start_time = seg_end_time;
        }

        // Fallback
        self.frames.last().unwrap().value.clone()
    }

    fn eval(&self, t: f64) -> V
    where
        V: Lerp,
    {
        if matches!(self.repeat, Repeat::PingPong | Repeat::PingPongNCycles(_))
            && !self.pingpong_forward
        {
            // Reverse direction: mirror time
            self.eval_forward(self.total - t)
        } else {
            self.eval_forward(t)
        }
    }
}

impl<V> Value<V> for Keyframes<V>
where
    V: Clone,
{
    fn value(&self) -> V {
        self.current_value.clone()
    }
}

impl<V> Animation for Keyframes<V>
where
    V: Lerp + Clone,
{
    fn step(&mut self, delta_time: f64) {
        if self.status == AnimationStatus::Ended {
            self.status = AnimationStatus::Idle;

            return;
        }

        if self.status == AnimationStatus::Idle {
            return;
        }

        if self.frames.len() <= 1 {
            self.current_value = self.frames.first().unwrap().value.clone();
            self.status = AnimationStatus::Ended;

            return;
        }

        // If total == 0, immediately finish (or repeat will cycle instantly)
        if self.total <= 0.0 {
            self.current_value = self.frames.last().unwrap().value.clone();
            self.on_cycle_end();

            return;
        }

        self.elapsed += delta_time.max(0.0);

        if self.elapsed >= self.total {
            // Snap to exact end of cycle before repeating/ending
            self.elapsed = self.total;
            self.current_value = self.eval(self.elapsed);
            self.on_cycle_end();

            if self.status != AnimationStatus::Ended {
                self.current_value = self.eval(self.elapsed);
            }
        } else {
            self.current_value = self.eval(self.elapsed);
            self.status = AnimationStatus::Updated;
        }
    }

    fn in_progress(&self) -> bool {
        self.status != AnimationStatus::Idle
    }
}
