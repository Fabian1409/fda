use clap::{arg, command};
use itertools::Itertools;
use plotters::coord::types::RangedCoordu32;
use plotters::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::read_to_string;
use std::str::FromStr;

const HLINE_PAD_X: u32 = 30;
const HLINE_PAD_Y: u32 = 100;
const EVENT_PAD_X: u32 = 50;
const FONT_SIZE: f32 = 20.0;

const SEND_EVENT: &str = "Send event";
const RECV_EVENT: &str = "Receive event";
const CHECKPOINT_EVENT: &str = "Checkpoint";

#[derive(Debug, Clone)]
struct Event {
    title: String,
    vec_clock: Vec<usize>,
    host: String,
    clock: usize,
    sender_clock: Option<(String, usize)>,
}

impl FromStr for Event {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (caption, host_clock) = s.split_once('\n').unwrap();
        let (host, clocks) = host_clock.split_once(' ').unwrap();
        let clocks: HashMap<String, usize> = serde_json::from_str(clocks).unwrap();
        let clock = *clocks.get(host).unwrap();
        let sender_clock: Vec<(String, usize)> = clocks
            .into_iter()
            .filter(|(h, _)| !h.to_owned().eq(host))
            .collect();
        let sender_clock = sender_clock.first().cloned();
        Ok(Event {
            title: caption.to_owned(),
            vec_clock: Vec::new(),
            host: host.to_owned(),
            clock,
            sender_clock,
        })
    }
}

fn draw_event(
    root: &DrawingArea<BitMapBackend, Cartesian2d<RangedCoordu32, RangedCoordu32>>,
    label: String,
    x: u32,
    y: u32,
    style: ShapeStyle,
) -> Result<(), Box<dyn Error>> {
    let event = EmptyElement::at((x, y))
        + Circle::new((0, 0), 5, style)
        + Text::new(label, (5, 5), ("sans-serif", FONT_SIZE).into_font());
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
    let hline = EmptyElement::at((HLINE_PAD_X, y))
        + Rectangle::new(
            [(0, 0), (w as i32 - 2 * HLINE_PAD_X as i32, 2)],
            ShapeStyle::from(&color).filled(),
        )
        + Text::new(
            String::from(label),
            (0, 5),
            ("sans-serif", FONT_SIZE).into_font(),
        );
    root.draw(&hline)?;
    Ok(())
}

fn draw_axis(
    root: &DrawingArea<BitMapBackend, Cartesian2d<RangedCoordu32, RangedCoordu32>>,
    n: usize,
    y: u32,
    w: u32,
) -> Result<(), Box<dyn Error>> {
    let axis = EmptyElement::at((HLINE_PAD_X, y))
        + Rectangle::new(
            [(0, 0), (w as i32 - 2 * HLINE_PAD_X as i32, 1)],
            ShapeStyle::from(&BLACK).filled(),
        );
    root.draw(&axis)?;
    for i in 0..n as u32 {
        let indicator = EmptyElement::at((i * EVENT_PAD_X + 2 * EVENT_PAD_X, y))
            + Rectangle::new([(0, -4), (1, 4)], ShapeStyle::from(&BLACK).filled())
            + Text::new(
                (i + 1).to_string(),
                (-4, 10),
                ("sans-serif", FONT_SIZE).into_font(),
            );
        root.draw(&indicator)?;
    }
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

fn visualize(events: &[Event], hosts: &BTreeMap<String, usize>) -> Result<(), Box<dyn Error>> {
    let w = *hosts.values().max().unwrap() as u32 * EVENT_PAD_X + 3 * EVENT_PAD_X;
    let h = hosts.len() as u32 * HLINE_PAD_Y + 2 * HLINE_PAD_Y;

    let root = BitMapBackend::new("out.png", (w, h)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root).build_cartesian_2d(0u32..w, h..0u32)?;
    let root = root.apply_coord_spec(Cartesian2d::<RangedCoordu32, RangedCoordu32>::new(
        0..w,
        0..h,
        (0..w as i32, 0..h as i32),
    ));

    let mut host_ys = HashMap::new();
    for (i, host) in hosts.keys().enumerate() {
        let y = HLINE_PAD_Y + HLINE_PAD_Y * i as u32;
        host_ys.insert(host, y);
        draw_hline(&root, host, y, w, BLACK)?;
    }

    for event in events.iter() {
        let host = event.host.clone();
        let x = event.clock as u32 * EVENT_PAD_X + EVENT_PAD_X;
        let y = *host_ys.get(&host).unwrap();
        let label = if event.title.eq(RECV_EVENT)
            | event.title.eq(SEND_EVENT)
            | event.title.eq(CHECKPOINT_EVENT)
        {
            String::from(event.title.chars().next().unwrap())
        } else {
            String::from("")
        };
        draw_event(&root, label, x, y, ShapeStyle::from(&BLACK).filled())?;

        if event.title.eq(RECV_EVENT) {
            let sender_clock = event.sender_clock.as_ref().unwrap();
            let send_x = sender_clock.1 as u32 * EVENT_PAD_X + EVENT_PAD_X;
            let send_y = *host_ys.get(&sender_clock.0).unwrap();
            draw_conn(&mut chart, (send_x, send_y), (x, y), BLACK)?;
        }
    }

    draw_axis(&root, *hosts.values().max().unwrap(), h - HLINE_PAD_Y, w)?;

    root.present()?;

    Ok(())
}

fn assign_vector_clocks(events: &mut Vec<Event>, hosts: &[String]) {
    let host_idxs: HashMap<String, usize> = hosts
        .iter()
        .enumerate()
        .map(|(i, host)| (host.clone(), i))
        .collect();

    let mut vclocks = HashMap::new();
    hosts.iter().for_each(|h| {
        vclocks.insert(h, vec![0usize; hosts.len()]);
    });

    // fails if log is not in cronological order
    let mut messages = HashMap::<(String, usize), Vec<usize>>::new();

    for event in events {
        let host = event.host.clone();
        // own vclock += 1
        vclocks.get_mut(&host).unwrap()[*host_idxs.get(&host).unwrap()] += 1;

        if event.title.eq(RECV_EVENT) {
            // get sender and sender_clock from recv event
            let sender_clock = event.sender_clock.as_ref().unwrap();

            // get sender vclock from messages
            let vec = messages.get(sender_clock).unwrap();

            // update all but own vclock with max(v_msg[j], v[j])
            hosts.iter().for_each(|h| {
                if !h.eq(&host) {
                    let j = *host_idxs.get(h).unwrap();
                    vclocks.get_mut(&host).unwrap()[j] = vclocks.get(&host).unwrap()[j].max(vec[j]);
                }
            });
        } else if event.title.eq(SEND_EVENT) {
            // store sender vclock in messages
            messages.insert(
                (host.clone(), event.clock),
                vclocks.get(&host).unwrap().clone(),
            );
        }

        event.vec_clock = vclocks.get(&host).unwrap().clone();
    }
}

fn are_concurrent(e1: &Event, e2: &Event) -> bool {
    e1.vec_clock
        .iter()
        .zip(e2.vec_clock.iter())
        .any(|(e1, e2)| e1 > e2)
        & e2.vec_clock
            .iter()
            .zip(e1.vec_clock.iter())
            .any(|(e2, e1)| e2 > e1)
}

fn count_concurrent_events(events: &[Event]) -> usize {
    let mut count = 0;
    for (i, e1) in events.iter().enumerate() {
        for e2 in events.iter().skip(i + 1) {
            if are_concurrent(e1, e2) {
                count += 1;
                // println!("{e1:?} ||| {e2:?}");
            }
        }
    }
    count
}

fn is_consistent_cut(cut: &[Event], host_events: &HashMap<String, Vec<Event>>) -> bool {
    for (host, events) in host_events {
        for event in events {
            if event.title.eq(RECV_EVENT) {
                let sender_clock = event.sender_clock.as_ref().unwrap();
                let send_event = host_events
                    .get(&sender_clock.0)
                    .unwrap()
                    .get(sender_clock.1 - 1)
                    .unwrap();
                // println!("send event {send_event:?} for recv event {event:?}");
                let cut_event_send = cut.iter().find(|e| e.host.eq(&sender_clock.0)).unwrap();
                let cut_event_recv = cut.iter().find(|e| e.host.eq(host)).unwrap();
                if (event.clock <= cut_event_recv.clock) & (send_event.clock > cut_event_send.clock)
                {
                    return false;
                }
            }
        }
    }
    true
}

fn find_recovery_line(events: &[Event], hosts: &[String], fail: &[String]) -> Vec<Event> {
    let mut host_events: HashMap<String, Vec<Event>> = hosts
        .iter()
        .map(|h| (h.clone(), Vec::<Event>::new()))
        .collect();
    events
        .iter()
        .for_each(|e| host_events.get_mut(&e.host).unwrap().push(e.clone()));

    let cuts: Vec<Vec<Event>> = host_events
        .clone()
        .into_values()
        .multi_cartesian_product()
        .collect();

    println!("Number of possible cuts {}", cuts.len());

    let consistent_cuts: Vec<Vec<Event>> = cuts
        .into_iter()
        .filter(|cut| is_consistent_cut(cut, &host_events))
        .collect();

    println!("Number of consistent cuts {}", consistent_cuts.len());

    let recovery_lines: Vec<Vec<Event>> = consistent_cuts
        .into_iter()
        .filter(|cut| {
            fail.iter().all(|f| {
                cut.iter()
                    .any(|e| e.title.eq(CHECKPOINT_EVENT) & e.host.eq(f))
            }) & cut.iter().all(|e| {
                e.title.eq(CHECKPOINT_EVENT) | e.clock.eq(&host_events.get(&e.host).unwrap().len())
            })
        })
        .collect();

    println!("Number of recovery lines {}", recovery_lines.len());

    let recovery_lines_sorted: Vec<Vec<Event>> = recovery_lines
        .into_iter()
        .sorted_by(|a, b| {
            let a_age = a.iter().fold(0, |acc, e| acc + e.clock);
            let b_age = b.iter().fold(0, |acc, e| acc + e.clock);
            Ord::cmp(&a_age, &b_age)
        })
        .collect();

    recovery_lines_sorted
        .last()
        .cloned()
        .expect("failed to find recovery line")
        .into_iter()
        .sorted_by(|a, b| Ord::cmp(&a.host, &b.host))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = command!()
        .arg(arg!(<FILE> "Path to log file with events"))
        .arg(arg!(-f --fail <FAIL> "Comma separated list of hosts e.g. Alice,Bob"))
        .get_matches();

    let path = matches.get_one::<String>("FILE").unwrap();
    let data = read_to_string(path)?;
    let events: Vec<&str> = data.lines().collect();
    let mut events: Vec<Event> = events
        .chunks(2)
        .map(|e| (e[0].to_owned() + "\n" + e[1]).parse())
        .collect::<Result<Vec<Event>, _>>()?;

    let mut hosts = BTreeMap::new();
    events.iter().for_each(|e| match hosts.get_mut(&e.host) {
        Some(num) => *num += 1,
        None => {
            hosts.insert(e.host.clone(), 1);
        }
    });

    visualize(&events, &hosts)?;

    let hosts: Vec<String> = hosts.into_keys().collect();

    assign_vector_clocks(&mut events, &hosts);

    let count = count_concurrent_events(&events);
    println!("Number of concurrent event pairs {count}");

    println!("Events with vector clocks");
    for (i, event) in events.iter().enumerate() {
        println!("{i}: {event:?}");
    }

    if let Some(fail) = matches.get_one::<String>("fail") {
        let fail: Vec<String> = fail.split(',').map(String::from).collect();
        println!("Following hosts will fail {fail:?}");

        let recovery_line = find_recovery_line(&events, &hosts, &fail);
        println!("Found recovery line {recovery_line:?}");
    } else {
        println!("No host will fail, no recovery line created");
    }

    Ok(())
}
