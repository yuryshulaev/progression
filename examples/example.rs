use std::{thread, time::Duration};

fn main() {
	// Default
	for _ in progression::bar(0..1_000) {
		thread::sleep(Duration::from_millis(1));
	}

	// Cargo style
	for _ in progression::bar_with_config(0..1_000, progression::Config::cargo()) {
		thread::sleep(Duration::from_millis(1));
	}

	// Unicode style
	for _ in progression::bar_with_config(0..1_000, progression::Config::unicode()) {
		thread::sleep(Duration::from_millis(1));
	}

	// Custom
	for _ in progression::bar_with_config(0..1_000, progression::Config { style: progression::Style::Mono('·'), ..Default::default() }) {
		thread::sleep(Duration::from_millis(1));
	}

	// Manual
	let items = vec![1, 2, 3, 4, 5];
	let bar = progression::Bar::new(items.len() as u64, progression::Config { prefix: "(items) ", ..progression::Config::cargo() });

	for _ in items {
		thread::sleep(Duration::from_millis(100));
		bar.inc(1);
	}

	bar.finish();
}
