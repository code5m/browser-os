//! Exercise the actual GLib used by the GTK3 plugin, including optimized builds.
#![cfg(target_os = "linux")]

use gtk::glib::{prelude::*, Variant};

fn sequence() -> Variant {
    println!(
        "GLIB_VARIANT_ITER_PROFILE={}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    ["α", "", "middle", "🦀", "尾"].to_variant()
}

#[test]
fn glib_variant_str_iter_forward_preserves_utf8_and_empty_values() {
    let value = sequence();
    let mut iter = value.array_iter_str().unwrap();
    assert_eq!(iter.len(), 5);
    for expected in ["α", "", "middle", "🦀", "尾"] {
        assert_eq!(iter.next(), Some(expected));
    }
    assert_eq!(iter.len(), 0);
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next_back(), None);
}

#[test]
fn glib_variant_str_iter_reverse_preserves_utf8_and_empty_values() {
    let value = sequence();
    let mut iter = value.array_iter_str().unwrap();
    for expected in ["尾", "🦀", "middle", "", "α"] {
        assert_eq!(iter.next_back(), Some(expected));
    }
    assert_eq!(iter.len(), 0);
    assert_eq!(iter.next_back(), None);
    assert_eq!(iter.next(), None);
}

#[test]
fn glib_variant_str_iter_nth_skips_and_exhausts() {
    let value = sequence();
    let mut iter = value.array_iter_str().unwrap();
    assert_eq!(iter.nth(1), Some(""));
    assert_eq!(iter.len(), 3);
    assert_eq!(iter.nth(1), Some("🦀"));
    assert_eq!(iter.next(), Some("尾"));
    assert_eq!(iter.nth(0), None);
    assert_eq!(value.array_iter_str().unwrap().nth(10), None);
}

#[test]
fn glib_variant_str_iter_nth_back_skips_and_exhausts() {
    let value = sequence();
    let mut iter = value.array_iter_str().unwrap();
    assert_eq!(iter.nth_back(1), Some("🦀"));
    assert_eq!(iter.next_back(), Some("middle"));
    assert_eq!(iter.nth_back(1), Some("α"));
    assert_eq!(iter.len(), 0);
    assert_eq!(iter.next_back(), None);
    assert_eq!(value.array_iter_str().unwrap().nth_back(10), None);
}

#[test]
fn glib_variant_str_iter_last_handles_nonempty_and_empty() {
    let value = sequence();
    assert_eq!(value.array_iter_str().unwrap().last(), Some("尾"));
    let empty = Vec::<String>::new().to_variant();
    assert_eq!(empty.array_iter_str().unwrap().last(), None);
}
