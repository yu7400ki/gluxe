// CSS timing functions as cubic béziers.
//
// Named keywords map to CSS-spec control points. Stored as an enum so
// `Copy`/`PartialEq` stay trivial; a `CubicBezier(f32,f32,f32,f32)` variant can be added later.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Easing {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "linear" => Some(Self::Linear),
            "ease" => Some(Self::Ease),
            "ease-in" => Some(Self::EaseIn),
            "ease-out" => Some(Self::EaseOut),
            "ease-in-out" => Some(Self::EaseInOut),
            _ => None,
        }
    }

    /// Evaluate the timing function at progress `t` (clamped to `[0, 1]`).
    pub(crate) fn eval(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let (x1, y1, x2, y2) = match self {
            Self::Linear => return t,
            Self::Ease => (0.25, 0.1, 0.25, 1.0),
            Self::EaseIn => (0.42, 0.0, 1.0, 1.0),
            Self::EaseOut => (0.0, 0.0, 0.58, 1.0),
            Self::EaseInOut => (0.42, 0.0, 0.58, 1.0),
        };
        cubic_bezier(x1, y1, x2, y2, t)
    }
}

/// Evaluate a CSS cubic-bezier timing function at `x` ∈ [0,1].
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    // B(u) = 3(1-u)²u·P1 + 3(1-u)u²·P2 + u³, P0=(0,0), P3=(1,1).
    let sample = |c1: f32, c2: f32, u: f32| {
        let v = 1.0 - u;
        3.0 * v * v * u * c1 + 3.0 * v * u * u * c2 + u * u * u
    };
    let sample_dx = |u: f32| {
        let v = 1.0 - u;
        3.0 * v * v * x1 + 6.0 * v * u * (x2 - x1) + 3.0 * u * u * (1.0 - x2)
    };

    // Newton-Raphson on the x-polynomial.
    let mut u = x;
    for _ in 0..8 {
        let err = sample(x1, x2, u) - x;
        if err.abs() < 1e-5 {
            return sample(y1, y2, u);
        }
        let dx = sample_dx(u);
        if dx.abs() < 1e-6 {
            break; // derivative too flat — fall back to bisection
        }
        u = (u - err / dx).clamp(0.0, 1.0);
    }

    // Bisection fallback (x is monotonic in u for valid CSS control points).
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..32 {
        u = (lo + hi) / 2.0;
        if sample(x1, x2, u) < x {
            lo = u;
        } else {
            hi = u;
        }
    }
    sample(y1, y2, u)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Easing; 5] = [
        Easing::Linear,
        Easing::Ease,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
    ];

    #[test]
    fn parse_known_keywords() {
        assert_eq!(Easing::parse("linear"), Some(Easing::Linear));
        assert_eq!(Easing::parse("ease"), Some(Easing::Ease));
        assert_eq!(Easing::parse("ease-in"), Some(Easing::EaseIn));
        assert_eq!(Easing::parse("ease-out"), Some(Easing::EaseOut));
        assert_eq!(Easing::parse("ease-in-out"), Some(Easing::EaseInOut));
        assert_eq!(Easing::parse("bounce"), None);
    }

    #[test]
    fn endpoints_are_exact() {
        for e in ALL {
            assert_eq!(e.eval(0.0), 0.0, "{e:?} at 0");
            assert_eq!(e.eval(1.0), 1.0, "{e:?} at 1");
        }
    }

    #[test]
    fn out_of_range_input_clamps() {
        for e in ALL {
            assert_eq!(e.eval(-0.5), 0.0);
            assert_eq!(e.eval(1.5), 1.0);
        }
    }

    #[test]
    fn linear_is_identity() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((Easing::Linear.eval(t) - t).abs() < 1e-6);
        }
    }

    #[test]
    fn curves_are_monotonic_nondecreasing() {
        for e in ALL {
            let mut prev = 0.0;
            for i in 0..=100 {
                let y = e.eval(i as f32 / 100.0);
                assert!(y >= prev - 1e-4, "{e:?} not monotonic at i={i}");
                prev = y;
            }
        }
    }

    #[test]
    fn ease_in_starts_slow_ease_out_starts_fast() {
        assert!(Easing::EaseIn.eval(0.25) < 0.25);
        assert!(Easing::EaseOut.eval(0.25) > 0.25);
        // ease-in-out is symmetric around 0.5
        let y = Easing::EaseInOut.eval(0.5);
        assert!((y - 0.5).abs() < 1e-3);
    }
}
