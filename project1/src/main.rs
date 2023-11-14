use clap::{arg, command};
use plotters::coord::types::RangedCoordu32;
use plotters::prelude::*;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::read_to_string;
use std::str::FromStr;

const HLINE_PAD_X: u32 = 30;
const HLINE_PAD_Y: u32 = 100;
const EVENT_PAD_X: u32 = 50;
const FONT_SIZE: f32 = 20.0;

#[derive(Debug)]
enum EventType {
    Event(String),
    Send,
    Receive,
}

impl FromStr for EventType {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Send event" => Ok(EventType::Send),
            "Receive event" => Ok(EventType::Receive),
            _ => Ok(EventType::Event(s.to_owned())),
        }
    }
}

#[derive(Debug)]
struct Event {
    event_type: EventType,
    host: String,
    clock: HashMap<String, usize>,
}

impl FromStr for Event {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (event_type, host_clock) = s.split_once('\n').unwrap();
        let (host, clock) = host_clock.split_once(' ').unwrap();
        Ok(Event {
            event_type: event_type.parse().unwrap(),
            host: host.to_owned(),
            clock: serde_json::from_str(clock).unwrap(),
        })
    }
}

fn draw_event(
    root: &DrawingArea<BitMapBackend, Cartesian2d<RangedCoordu32, RangedCoordu32>>,
    label: &str,
    x: u32,
    y: u32,
    color: RGBColor,
) -> Result<(), Box<dyn Error>> {
    let event = EmptyElement::at((x, y))
        + Circle::new((0, 0), 3, ShapeStyle::from(&color).filled())
        + Text::new(
            String::from(label),
            (5, 5),
            ("sans-serif", FONT_SIZE).into_font(),
        );
    root.draw(&event)?;
    Ok(())
}

fn draw_hline(
    root: &DrawingArea<BitMapBackend, Cartesian2d<RangedCoordu32, RangedCoordu32>>,
    label: &str,
    y: u32,
    w: u32,
    color: RGBColor,
) -> Result<(), Box<dyn Error>> {
    let event = EmptyElement::at((HLINE_PAD_X, y))
        + Rectangle::new(
            [(0, 0), (w as i32 - 2 * HLINE_PAD_X as i32, 1)],
            ShapeStyle::from(&color).filled(),
        )
        + Text::new(
            String::from(label),
            (-20, 5),
            ("sans-serif", FONT_SIZE).into_font(),
        );
    root.draw(&event)?;
    Ok(())
}

fn draw_conn(
    chart: &mut ChartContext<BitMapBackend, Cartesian2d<RangedCoordu32, RangedCoordu32>>,
    from: (u32, u32),
    to: (u32, u32),
    color: RGBColor,
) -> Result<(), Box<dyn Error>> {
    chart.draw_series(LineSeries::new(vec![from, to], color))?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!()
        .arg(arg!(<LOG_FILE> "Path to log file with events"))
        .get_matches();

    let path = matches.get_one::<String>("LOG_FILE").unwrap();
    let data = read_to_string(path)?;
    let events: Vec<&str> = data.lines().collect();
    let events: Vec<Event> = events
        .chunks(2)
        .map(|e| (e[0].to_owned() + "\n" + e[1]).parse())
        .collect::<Result<Vec<Event>, _>>()?;

    let mut hosts = HashSet::new();
    events.iter().for_each(|e| {
        hosts.insert(e.host.clone());
    });

    let w = events.len() as u32 * EVENT_PAD_X + EVENT_PAD_X;
    let h = hosts.len() as u32 * HLINE_PAD_Y + HLINE_PAD_Y;

    let root = BitMapBackend::new("out.png", (w, h)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root).build_cartesian_2d(0u32..w, h..0u32)?;
    let root = root.apply_coord_spec(Cartesian2d::<RangedCoordu32, RangedCoordu32>::new(
        0..w,
        0..h,
        (0..w as i32, 0..h as i32),
    ));

    let mut host_ys = HashMap::new();
    for (i, host) in hosts.iter().enumerate() {
        let y = HLINE_PAD_Y + HLINE_PAD_Y * i as u32;
        host_ys.insert(host, y);
        draw_hline(&root, host, y, w, BLACK)?;
    }

    for event in events {
        let host = event.host;
        let x = *event.clock.get(&host).unwrap() as u32 * EVENT_PAD_X + EVENT_PAD_X;
        let y = *host_ys.get(&host).unwrap();
        draw_event(&root, "A", x, y, BLACK)?;
    }

    draw_conn(&mut chart, (0, 0), (20, 20), RED)?;
    draw_event(&root, "A", 300, 300, BLACK)?;
    root.present()?;
    Ok(())
}
