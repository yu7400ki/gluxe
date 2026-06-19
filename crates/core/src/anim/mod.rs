// Style transitions: Rust-side interpolation of style-prop changes.
//
// On `UpdateProps`, animatable fields covered by a `transition` spec start an
// [`ActiveTransition`] instead of applying instantly. `tick` advances them
// each GPUI frame; `overlay` overlays `current` onto the cloned style before
// `apply_style_props`, so render always sees the interpolated value. Finished
// entries are dropped and the element renders its raw target props.
//
// State lives in a thread_local (not inside `Tree`/`Element`) to keep
// model.rs a pure command applier. Everything runs on the single Boa/GPUI thread.

mod easing;
mod fields;

use std::cell::RefCell;

use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) use easing::Easing;
pub(crate) use fields::{
    AnimValue, FieldId, diff_animatable, field_id_from_name, read_field, write_field,
};

use crate::model::{ElementId, Props, StyleFields};

/// Which style field(s) a transition spec applies to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TransitionProperty {
    /// Every animatable field (`property: "all"`).
    All,
    /// A single named field.
    Field(FieldId),
}

/// One parsed `transition` declaration from the JS style object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransitionSpec {
    pub(crate) property: TransitionProperty,
    pub(crate) duration_ms: f32,
    pub(crate) delay_ms: f32,
    pub(crate) easing: Easing,
}

/// A field currently animating on one element.
#[derive(Debug, Clone)]
struct ActiveTransition {
    field: FieldId,
    from: AnimValue,
    to: AnimValue,
    /// Absolute start time (`now + delay` at creation), fractional ms on the
    /// Boa monotonic clock.
    start_ms: f64,
    duration_ms: f32,
    easing: Easing,
    /// Value as of the last `tick`. Render reads only this, never the clock,
    /// so all nodes within one frame see a consistent timestamp.
    current: AnimValue,
}

thread_local! {
    static ACTIVE: RefCell<FxHashMap<ElementId, Vec<ActiveTransition>>> =
        RefCell::new(FxHashMap::default());
}

/// Find the spec governing `field`: specific `Field` beats `All`; within the
/// same specificity the last declaration wins (CSS semantics).
fn matching_spec(specs: &[TransitionSpec], field: FieldId) -> Option<&TransitionSpec> {
    let mut found: Option<&TransitionSpec> = None;
    let mut found_specific = false;
    for spec in specs {
        match spec.property {
            TransitionProperty::Field(f) if f == field => {
                found = Some(spec);
                found_specific = true;
            }
            TransitionProperty::All if !found_specific => found = Some(spec),
            _ => {}
        }
    }
    found
}

/// Start, replace, or cancel transitions for each changed animatable field.
/// Called from `flush_commands` *before* `apply_command` swaps the props in.
///
/// `now_ms` is `None` only before the Boa context exists (initial flush) —
/// all changes then apply instantly.
pub(crate) fn on_props_update(
    id: ElementId,
    old_style: &StyleFields,
    new_props: &Props,
    now_ms: Option<f64>,
) {
    if new_props.transitions.is_empty() {
        // Spec removed: drop in-flight entries so the element snaps to raw style.
        ACTIVE.with(|a| {
            a.borrow_mut().remove(&id);
        });
        return;
    }
    let Some(now) = now_ms else {
        ACTIVE.with(|a| {
            a.borrow_mut().remove(&id);
        });
        return;
    };

    let changed = diff_animatable(old_style, &new_props.style);
    if changed.is_empty() {
        return;
    }

    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let entries = active.entry(id).or_default();
        for field in changed {
            // Fields not in the diff stay untouched — changing `width` must not restart a running `opacity` transition.
            let existing = entries.iter().position(|e| e.field == field);
            let spec = matching_spec(&new_props.transitions, field);
            let Some(spec) = spec else {
                // Changed but not covered by any spec → instant jump.
                if let Some(i) = existing {
                    entries.remove(i);
                }
                continue;
            };
            if spec.duration_ms <= 0.0 {
                if let Some(i) = existing {
                    entries.remove(i);
                }
                continue;
            }
            // Interruption: restart from the mid-flight interpolated value.
            let from = existing
                .map(|i| entries[i].current)
                .or_else(|| read_field(old_style, field));
            let to = read_field(&new_props.style, field);
            let (Some(from), Some(to)) = (from, to) else {
                // Added/removed endpoint → not interpolable, jump.
                if let Some(i) = existing {
                    entries.remove(i);
                }
                continue;
            };
            if AnimValue::lerp(from, to, 0.0).is_none() {
                // Unit/kind mismatch (or auto) → jump.
                if let Some(i) = existing {
                    entries.remove(i);
                }
                continue;
            }
            let entry = ActiveTransition {
                field,
                from,
                to,
                start_ms: now + spec.delay_ms as f64,
                duration_ms: spec.duration_ms,
                easing: spec.easing,
                current: from,
            };
            match existing {
                Some(i) => entries[i] = entry,
                None => entries.push(entry),
            }
        }
        if entries.is_empty() {
            active.remove(&id);
        }
    });
}

/// Advance all active transitions to `now`, dropping finished ones. Returns
/// ids of every node with an active entry — including those that just finished,
/// so their final raw value gets one more render.
pub(crate) fn tick(now_ms: f64) -> FxHashSet<ElementId> {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let mut dirty = FxHashSet::default();
        active.retain(|id, entries| {
            dirty.insert(*id);
            entries.retain_mut(|e| {
                let t = if now_ms < e.start_ms {
                    0.0 // delay phase: hold the old value (CSS behaviour)
                } else if e.duration_ms <= 0.0 {
                    1.0
                } else {
                    (((now_ms - e.start_ms) / e.duration_ms as f64) as f32).clamp(0.0, 1.0)
                };
                // lerp is always Some here — interpolability was checked when
                // the entry was created and from/to never change afterwards.
                if let Some(v) = AnimValue::lerp(e.from, e.to, e.easing.eval(t)) {
                    e.current = v;
                }
                t < 1.0
            });
            !entries.is_empty()
        });
        dirty
    })
}

/// Overlay active transition values onto a cloned style before `apply_style_props`.
pub(crate) fn overlay(id: ElementId, style: &mut StyleFields) {
    ACTIVE.with(|active| {
        if let Some(entries) = active.borrow().get(&id) {
            for e in entries {
                write_field(style, e.field, e.current);
            }
        }
    });
}

/// Whether any transition is in flight — the pump keeps arming GPUI frames
/// while this is true.
pub(crate) fn has_active() -> bool {
    ACTIVE.with(|a| !a.borrow().is_empty())
}

/// Drop all state for a removed node (called on `DetachDeleted`).
pub(crate) fn remove_node(id: ElementId) {
    ACTIVE.with(|a| {
        a.borrow_mut().remove(&id);
    });
}

/// Drop every in-flight transition (dev-mode full reload: all old ids die).
#[cfg(debug_assertions)]
pub(crate) fn clear() {
    ACTIVE.with(|a| a.borrow_mut().clear());
}

/// Register this module's node-lifecycle cleanup with the lifecycle seam.
pub(crate) fn register_lifecycle() {
    crate::lifecycle::on_detach(remove_node);
    #[cfg(debug_assertions)]
    crate::lifecycle::on_reload(clear);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LengthValue;

    fn reset() {
        ACTIVE.with(|a| a.borrow_mut().clear());
    }

    fn spec_all(duration: f32) -> TransitionSpec {
        TransitionSpec {
            property: TransitionProperty::All,
            duration_ms: duration,
            delay_ms: 0.0,
            easing: Easing::Linear,
        }
    }

    fn props_with(width: f32, transitions: Vec<TransitionSpec>) -> Props {
        let mut p = Props::default();
        p.style.width = Some(LengthValue::Px(width));
        p.transitions = transitions;
        p
    }

    fn style_with_width(width: f32) -> StyleFields {
        let mut s = StyleFields::default();
        s.width = Some(LengthValue::Px(width));
        s
    }

    fn current_width(id: ElementId) -> Option<LengthValue> {
        let mut s = StyleFields::default();
        overlay(id, &mut s);
        s.width
    }

    #[test]
    fn transition_starts_progresses_and_finishes() {
        reset();
        let old = style_with_width(0.0);
        let new = props_with(100.0, vec![spec_all(100.0)]);
        on_props_update(1, &old, &new, Some(1000.0));
        assert!(has_active());

        // t=0: holds the old value.
        let dirty = tick(1000.0);
        assert!(dirty.contains(&1));
        assert_eq!(current_width(1), Some(LengthValue::Px(0.0)));

        // Halfway.
        tick(1050.0);
        assert_eq!(current_width(1), Some(LengthValue::Px(50.0)));

        // Done: entry removed, completion frame still reported dirty.
        let dirty = tick(1100.0);
        assert!(dirty.contains(&1));
        assert!(!has_active());
        assert_eq!(current_width(1), None); // overlay is a no-op now
    }

    #[test]
    fn interruption_restarts_from_current_value() {
        reset();
        let old = style_with_width(0.0);
        on_props_update(
            1,
            &old,
            &props_with(100.0, vec![spec_all(100.0)]),
            Some(0.0),
        );
        tick(50.0); // current = 50

        // Reverse direction mid-flight: new target 0, must start from 50.
        let old = style_with_width(100.0);
        on_props_update(1, &old, &props_with(0.0, vec![spec_all(100.0)]), Some(50.0));
        tick(50.0);
        assert_eq!(current_width(1), Some(LengthValue::Px(50.0)));
        tick(100.0);
        assert_eq!(current_width(1), Some(LengthValue::Px(25.0)));
        let _ = tick(150.0);
        assert!(!has_active());
    }

    #[test]
    fn specific_spec_beats_all_and_last_wins() {
        let specs = vec![
            TransitionSpec {
                property: TransitionProperty::Field(FieldId::width),
                duration_ms: 300.0,
                delay_ms: 0.0,
                easing: Easing::Linear,
            },
            spec_all(100.0),
            TransitionSpec {
                property: TransitionProperty::Field(FieldId::width),
                duration_ms: 500.0,
                delay_ms: 0.0,
                easing: Easing::Linear,
            },
        ];
        // width: specific beats the `all` in between; last specific wins.
        let m = matching_spec(&specs, FieldId::width).unwrap();
        assert_eq!(m.duration_ms, 500.0);
        // opacity: only `all` matches.
        let m = matching_spec(&specs, FieldId::opacity).unwrap();
        assert_eq!(m.duration_ms, 100.0);
    }

    #[test]
    fn uncovered_field_jumps_and_unrelated_entry_survives() {
        reset();
        // Start a width transition.
        let old = style_with_width(0.0);
        let specs = vec![TransitionSpec {
            property: TransitionProperty::Field(FieldId::width),
            duration_ms: 100.0,
            delay_ms: 0.0,
            easing: Easing::Linear,
        }];
        on_props_update(1, &old, &props_with(100.0, specs.clone()), Some(0.0));
        tick(50.0);

        // Now change only opacity (not covered) — width entry must keep running.
        let mut old = style_with_width(100.0);
        old.opacity = Some(1.0);
        let mut new = props_with(100.0, specs);
        new.style.opacity = Some(0.0);
        on_props_update(1, &old, &new, Some(50.0));
        tick(75.0);
        assert_eq!(current_width(1), Some(LengthValue::Px(75.0)));
        let mut s = StyleFields::default();
        overlay(1, &mut s);
        assert_eq!(s.opacity, None); // opacity jumped, no entry
    }

    #[test]
    fn delay_holds_old_value_then_animates() {
        reset();
        let old = style_with_width(0.0);
        let mut spec = spec_all(100.0);
        spec.delay_ms = 100.0;
        on_props_update(1, &old, &props_with(100.0, vec![spec]), Some(0.0));

        tick(50.0); // still in delay
        assert_eq!(current_width(1), Some(LengthValue::Px(0.0)));
        tick(150.0); // 50ms into the 100ms run
        assert_eq!(current_width(1), Some(LengthValue::Px(50.0)));
        tick(200.0);
        assert!(!has_active());
    }

    #[test]
    fn zero_duration_and_unit_mismatch_jump() {
        reset();
        let old = style_with_width(0.0);
        on_props_update(1, &old, &props_with(100.0, vec![spec_all(0.0)]), Some(0.0));
        assert!(!has_active());

        // px → percent: not interpolable.
        let old = style_with_width(100.0);
        let mut new = props_with(0.0, vec![spec_all(100.0)]);
        new.style.width = Some(LengthValue::Percent(50.0));
        on_props_update(1, &old, &new, Some(0.0));
        assert!(!has_active());
    }

    #[test]
    fn spec_removed_mid_flight_snaps() {
        reset();
        let old = style_with_width(0.0);
        on_props_update(
            1,
            &old,
            &props_with(100.0, vec![spec_all(100.0)]),
            Some(0.0),
        );
        assert!(has_active());

        let old = style_with_width(100.0);
        on_props_update(1, &old, &props_with(200.0, vec![]), Some(50.0));
        assert!(!has_active());
    }

    #[test]
    fn no_clock_means_instant() {
        reset();
        let old = style_with_width(0.0);
        on_props_update(1, &old, &props_with(100.0, vec![spec_all(100.0)]), None);
        assert!(!has_active());
    }

    #[test]
    fn remove_node_drops_state() {
        reset();
        let old = style_with_width(0.0);
        on_props_update(
            7,
            &old,
            &props_with(100.0, vec![spec_all(100.0)]),
            Some(0.0),
        );
        assert!(has_active());
        remove_node(7);
        assert!(!has_active());
    }
}
