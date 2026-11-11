//! Exact rational time base (`254_016_000_000` ticks per second) shared across ArtCraft timelines.
//!
//! Matches `2^10 * 3^4 * 5^6 * 7^2 * 11 = 254_016_000_000` ticks/s without linking sibling
//! `filmcraft-*` crates in-process.

use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Exact sub-sample / sub-frame ticks per second (`254_016_000_000`).
pub const TICKS_PER_SECOND: i64 = 254_016_000_000;

/// Exact timeline timestamp or duration in `1 / 254_016_000_000` second ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tick(pub i64);

impl Tick {
    pub const ZERO: Self = Self(0);
    pub const MIN: Self = Self(i64::MIN);
    pub const MAX: Self = Self(i64::MAX);

    pub fn from_seconds_f64(seconds: f64) -> Self {
        if !seconds.is_finite() {
            return Self::ZERO;
        }
        let scaled = (seconds * (TICKS_PER_SECOND as f64)).round();
        if scaled >= (i64::MAX as f64) {
            Self::MAX
        } else if scaled <= (i64::MIN as f64) {
            Self::MIN
        } else {
            Self(scaled as i64)
        }
    }

    pub fn from_units(value: i64, per_second: i64) -> Self {
        if per_second == 0 {
            return Self::ZERO;
        }
        let ticks = (i128::from(value) * i128::from(TICKS_PER_SECOND)) / i128::from(per_second);
        Self(ticks.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
    }

    pub fn to_units_floor(self, per_second: i64) -> i64 {
        if per_second <= 0 {
            return 0;
        }
        let div = (i128::from(self.0) * i128::from(per_second)).div_euclid(i128::from(TICKS_PER_SECOND));
        div.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }

    pub fn mul_ratio(self, num: i64, den: i64) -> Self {
        if den == 0 {
            return Self::ZERO;
        }
        let prod = i128::from(self.0) * i128::from(num) / i128::from(den);
        Self(prod.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
    }

    pub const fn abs(self) -> Self {
        Self(self.0.saturating_abs())
    }

    pub fn seconds(self) -> f64 {
        (self.0 as f64) / (TICKS_PER_SECOND as f64)
    }
}

impl Add for Tick {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Tick {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for Tick {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Tick {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl Neg for Tick {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(self.0.saturating_neg())
    }
}

/// Exact rational frame rate (`num / den` frames per second).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRate {
    pub num: i64,
    pub den: i64,
}

const fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

impl FrameRate {
    pub const FPS_23_976: Self = Self { num: 24_000, den: 1_001 };
    pub const FPS_24: Self = Self { num: 24, den: 1 };
    pub const FPS_25: Self = Self { num: 25, den: 1 };
    pub const FPS_29_97: Self = Self { num: 30_000, den: 1_001 };
    pub const FPS_30: Self = Self { num: 30, den: 1 };
    pub const FPS_48: Self = Self { num: 48, den: 1 };
    pub const FPS_50: Self = Self { num: 50, den: 1 };
    pub const FPS_59_94: Self = Self { num: 60_000, den: 1_001 };
    pub const FPS_60: Self = Self { num: 60, den: 1 };
    pub const FPS_119_88: Self = Self { num: 120_000, den: 1_001 };
    pub const FPS_120: Self = Self { num: 120, den: 1 };

    pub const fn new(num: i64, den: i64) -> Self {
        if num <= 0 || den <= 0 {
            return Self { num, den };
        }
        let g = gcd_u64(num as u64, den as u64) as i64;
        Self { num: num / g, den: den / g }
    }

    pub const fn sane(self) -> Self {
        if self.num > 0 && self.den > 0 { Self::new(self.num, self.den) } else { Self::FPS_30 }
    }

    pub fn as_f64(self) -> f64 {
        let r = self.sane();
        (r.num as f64) / (r.den as f64)
    }

    pub fn from_f64(fps: f64) -> Self {
        if !fps.is_finite() || fps <= 0.0 {
            return Self::FPS_30;
        }
        for cand in [
            Self::FPS_23_976,
            Self::FPS_24,
            Self::FPS_25,
            Self::FPS_29_97,
            Self::FPS_30,
            Self::FPS_48,
            Self::FPS_50,
            Self::FPS_59_94,
            Self::FPS_60,
            Self::FPS_119_88,
            Self::FPS_120,
        ] {
            if (cand.as_f64() - fps).abs() < 0.01 {
                return cand;
            }
        }
        Self::new(((fps * 1000.0).round() as i64).max(1), 1000)
    }

    pub fn ticks_per_frame(self) -> Tick {
        let r = self.sane();
        Tick::from_units(r.den, r.num)
    }

    /// Floored frame index corresponding to `tick` at this rational frame rate.
    pub fn frame_at(self, tick: Tick) -> i64 {
        let r = self.sane();
        let num = i128::from(tick.0) * i128::from(r.num);
        let den = i128::from(TICKS_PER_SECOND) * i128::from(r.den);
        num.div_euclid(den).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }

    /// Exact [`Tick`] at the start of 0-based `frame`.
    pub fn tick_at_frame(self, frame: i64) -> Tick {
        let r = self.sane();
        let num = i128::from(frame) * i128::from(TICKS_PER_SECOND) * i128::from(r.den);
        let ticks = num / i128::from(r.num);
        Tick(ticks.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
    }
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::FPS_30
    }
}

/// Half-open timeline span `[start, start + duration)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Tick,
    pub duration: Tick,
}

impl TimeRange {
    pub const fn new(start: Tick, duration: Tick) -> Self {
        Self { start, duration }
    }

    pub fn end(self) -> Tick {
        self.start + self.duration
    }
}

pub fn parse_tick(v: &Value) -> Option<Tick> {
    if let Some(t) = v.get("ticks").and_then(Value::as_i64) {
        return Some(Tick(t));
    }
    if let Some(n) = v.as_i64() {
        if n == 0 || n.unsigned_abs() >= (TICKS_PER_SECOND as u64 / 10_000) {
            return Some(Tick(n));
        }
        return Some(Tick::from_units(n, 1));
    }
    v.as_f64().filter(|f| f.is_finite()).map(Tick::from_seconds_f64)
}

pub fn parse_raw_tick(v: &Value) -> Option<Tick> {
    v.get("ticks").and_then(Value::as_i64).or_else(|| v.as_i64()).map(Tick)
}

pub fn parse_frame_rate(v: &Value) -> Option<FrameRate> {
    if let (Some(num), Some(den)) = (v.get("num").and_then(Value::as_i64), v.get("den").and_then(Value::as_i64)) {
        return (num > 0 && den > 0).then(|| FrameRate::new(num, den));
    }
    if let Some(s) = v.as_str() {
        let trimmed = s.trim().trim_end_matches("fps").trim();
        if let Some((num_s, den_s)) = trimmed.split_once('/')
            && let (Ok(num), Ok(den)) = (num_s.trim().parse::<i64>(), den_s.trim().parse::<i64>())
        {
            return (num > 0 && den > 0).then(|| FrameRate::new(num, den));
        }
        return match trimmed {
            "23.976" | "23.98" => Some(FrameRate::FPS_23_976),
            "24" => Some(FrameRate::FPS_24),
            "25" => Some(FrameRate::FPS_25),
            "29.97" => Some(FrameRate::FPS_29_97),
            "30" => Some(FrameRate::FPS_30),
            "48" => Some(FrameRate::FPS_48),
            "50" => Some(FrameRate::FPS_50),
            "59.94" => Some(FrameRate::FPS_59_94),
            "60" => Some(FrameRate::FPS_60),
            "119.88" => Some(FrameRate::FPS_119_88),
            "120" => Some(FrameRate::FPS_120),
            _ => trimmed.parse::<i64>().ok().filter(|&n| n > 0).map(|n| FrameRate::new(n, 1)),
        };
    }
    if let Some(n) = v.as_i64() {
        return (n > 0).then(|| FrameRate::new(n, 1));
    }
    v.as_f64().filter(|f| f.is_finite() && *f > 0.0).map(FrameRate::from_f64)
}

pub fn frame_rate_json(rate: FrameRate) -> Value {
    let r = rate.sane();
    json!({ "num": r.num, "den": r.den })
}
