use std::{io::{stderr, Write}, fmt::Display, time::Instant, sync::atomic::{AtomicU64, Ordering::SeqCst}};

#[cfg(feature = "num-format")]
use num_format::{Locale, ToFormattedString, ToFormattedStr};

pub enum Style {
	Mono(char),
	Edged(char, char),
}

impl Style {
	fn bar_char(&self) -> char {
		match *self { Self::Mono(c) | Self::Edged(c, _) => c }
	}

	fn edge_char(&self) -> char {
		match *self { Self::Mono(c) | Self::Edged(_, c) => c }
	}
}

pub struct Config<'a> {
	pub width: Option<u64>,
	pub default_width: u64,
	pub delimiters: (char, char),
	pub style: Style,
	pub space_char: char,
	pub prefix: &'a str,
	pub unit: &'a str,
	pub num_width: usize,
	pub throttle_millis: u64,
}

impl Config<'_> {
	#[inline]
	pub fn ascii() -> Self {
		Self { style: Style::Mono('#'), ..Default::default() }
	}

	#[inline]
	pub fn unicode() -> Self {
		Self { style: Style::Mono('█'), ..Default::default() }
	}

	#[inline]
	pub fn cargo() -> Self {
		Self { style: Style::Edged('=', '>'), ..Default::default() }
	}
}

impl Default for Config<'_> {
	fn default() -> Self {
		Self {
			width: None,
			default_width: 80,
			delimiters: ('[', ']'),
			style: Style::Mono('#'),
			space_char: ' ',
			prefix: "",
			unit: "",
			num_width: 0,
			throttle_millis: 10,
		}
	}
}

#[inline]
pub fn bar<I: ExactSizeIterator>(iter: I) -> impl Iterator<Item = I::Item> {
	bar_with_config(iter, Config::default())
}

#[inline]
pub fn bar_with_config<I: ExactSizeIterator>(iter: I, config: Config) -> std::iter::Map<I, impl FnMut(I::Item) -> I::Item + '_> {
	let bar = Bar::new(iter.size_hint().0.try_into().unwrap(), config);

	iter.map(move |x| {
		bar.inc(1);
		x
	})
}

pub struct Bar<'a> {
	config: Config<'a>,
	len: u64,
	pos: AtomicU64,
	len_str: String,
	bar_width: u64,
	start_time: Instant,
	last_update: AtomicU64,
}

impl<'a> Bar<'a> {
	#[inline]
	pub fn new(len: u64, mut config: Config<'a>) -> Self {
		let len_str = format_number(len);
		config.num_width = config.num_width.max(len_str.len());
		#[cfg(feature = "terminal_size")]
		{ config.width = config.width.or_else(|| Some(u64::from(terminal_size::terminal_size()?.0.0))) }
		let bar_width = config.width.unwrap_or(config.default_width) - 35 - (config.prefix.len() + config.unit.len() + config.num_width * 2) as u64
			- if config.unit.is_empty() { 0 } else { 1 };
		Self { config, bar_width, len, pos: AtomicU64::new(0), len_str, start_time: Instant::now(), last_update: AtomicU64::new(0) }
	}

	fn print(&self) -> std::io::Result<()> {
		let mut stderr = stderr().lock();
		let pos = self.pos.load(SeqCst);
		assert!(pos <= self.len);
		let ratio = (pos as f64) / (self.len as f64);
		let progress_width = (ratio * (self.bar_width as f64)).round() as u64;
		let secs_per_step = self.start_time.elapsed().as_secs_f64() / (pos as f64);
		let eta = Time(((self.len.saturating_sub(pos) as f64) * secs_per_step).ceil() as u64);

		write!(stderr, "\r{} {} {:>num_width$} / {:>num_width$}{}{} {}", self.config.prefix, Time(self.start_time.elapsed().as_secs()), format_number(pos),
			self.len_str, if self.config.unit.is_empty() { "" } else { " " }, self.config.unit, self.config.delimiters.0, num_width = self.config.num_width)?;
		write_iter(&mut stderr, std::iter::repeat(self.config.style.bar_char()).take(progress_width as usize))?;
		write!(stderr, "{}", if pos == self.len { self.config.style.bar_char() } else { self.config.style.edge_char() })?;
		write_iter(&mut stderr, std::iter::repeat(self.config.space_char).take((self.bar_width - progress_width) as usize))?;
		write!(stderr, "{} {:3.0}% ETA {eta}\r", self.config.delimiters.1, ratio * 100.)?;
		stderr.flush()?;
		Ok(())
	}

	#[inline]
	pub fn inc(&self, delta: u64) {
		self.pos.fetch_add(delta, SeqCst);
		let elapsed = self.elapsed_millis();
		let last_update = self.last_update.load(SeqCst);

		if elapsed - last_update > self.config.throttle_millis && self.last_update.compare_exchange(last_update, elapsed, SeqCst, SeqCst).is_ok() {
			self.print().unwrap();
		}
	}

	#[inline]
	pub fn finish(self) {
		drop(self);
	}

	fn elapsed_millis(&self) -> u64 {
		self.start_time.elapsed().as_millis().try_into().unwrap()
	}
}

impl Drop for Bar<'_> {
	#[inline]
	fn drop(&mut self) {
		self.print().unwrap();
		eprintln!();
	}
}

fn write_iter<W, I>(w: &mut W, mut iter: I) -> std::io::Result<()>
where
	W: Write,
	I: Iterator,
	I::Item: Display,
{
	iter.try_for_each(|x| write!(w, "{x}"))
}

#[cfg(feature = "num-format")]
fn format_number<T: ToFormattedStr>(number: T) -> String {
	number.to_formatted_string(&Locale::en)
}

#[cfg(not(feature = "num-format"))]
fn format_number<T: Display>(number: T) -> String {
	number.to_string()
}

struct Time(u64);

impl Display for Time {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let hours = self.0 / 3600;

		if hours > 99 {
			write!(f, "??:??:??")
		} else {
			let mins = (self.0 / 60) % 60;
			let secs = self.0 % 60;
			write!(f, "{hours:02}:{mins:02}:{secs:02}")
		}
	}
}
