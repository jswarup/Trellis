// sdc_shard.rs ------------------------------------------------------------------------------------------------
use crate::rube::sdc::{
    ClockConstraint, DelayKind, DelayType, PathConstraint, PathKind, PortDelayConstraint,
    SdcConstraints,
};
use crate::silo::Stash;

//-------------------------------------------------------------------------------------------------

/// Tokenizes an SDC command line handling quotes, brackets (`[get_ports X]`), and braces (`{0 5}`).
pub fn tokenize_sdc_line(line: &str) -> Vec<String>
{
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || chars[i] == '#' {
            break;
        }
        if chars[i] == '[' {
            let start = i;
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '[' { depth += 1; }
                else if chars[i] == ']' {
                    depth -= 1;
                    if depth == 0 { i += 1; break; }
                }
                i += 1;
            }
            let inner: String = chars[start..i].iter().collect();
            let unwrapped = unwrap_bracket_target(&inner);
            tokens.push(unwrapped);
        } else if chars[i] == '{' || chars[i] == '"' {
            let close = if chars[i] == '{' { '}' } else { '"' };
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != close {
                i += 1;
            }
            let val: String = chars[start..i].iter().collect();
            if i < chars.len() { i += 1; }
            tokens.push(val);
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '#' {
                i += 1;
            }
            let tok: String = chars[start..i].iter().collect();
            tokens.push(tok);
        }
    }
    tokens
}

//-------------------------------------------------------------------------------------------------

fn unwrap_bracket_target(expr: &str) -> String
{
    let trimmed = expr.trim_matches(|c| c == '[' || c == ']').trim();
    if let Some(target) = trimmed.strip_prefix("get_ports ") {
        target.trim_matches(|c| c == '{' || c == '}' || c == '"').trim().to_string()
    } else if let Some(target) = trimmed.strip_prefix("get_pins ") {
        target.trim_matches(|c| c == '{' || c == '}' || c == '"').trim().to_string()
    } else if let Some(target) = trimmed.strip_prefix("get_clocks ") {
        target.trim_matches(|c| c == '{' || c == '}' || c == '"').trim().to_string()
    } else {
        trimmed.to_string()
    }
}

//-------------------------------------------------------------------------------------------------

fn parse_create_clock(tokens: &[String]) -> Result<ClockConstraint, String>
{
    let mut name = String::new();
    let mut period = 0.0f64;
    let mut rise = 0.0f64;
    let mut fall = 0.0f64;
    let mut port = String::new();
    let mut has_waveform = false;

    let mut i = 1usize;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-name" => {
                i += 1;
                name = tokens.get(i).ok_or("Missing clock name")?.clone();
            }
            "-period" => {
                i += 1;
                let s = tokens.get(i).ok_or("Missing clock period")?;
                period = s.parse::<f64>().map_err(|e| format!("Invalid period: {}", e))?;
            }
            "-waveform" => {
                i += 1;
                let wf_str = tokens.get(i).ok_or("Missing waveform values")?;
                let parts: Vec<&str> = wf_str.split_whitespace().collect();
                if parts.len() >= 2 {
                    rise = parts[0].parse().unwrap_or(0.0);
                    fall = parts[1].parse().unwrap_or(0.0);
                    has_waveform = true;
                }
            }
            _ => {
                if !tokens[i].starts_with('-') {
                    port = tokens[i].clone();
                }
            }
        }
        i += 1;
    }
    if !has_waveform && period > 0.0 {
        fall = period / 2.0;
    }
    if name.is_empty() {
        name = if !port.is_empty() { port.clone() } else { "clk".to_string() };
    }
    Ok(ClockConstraint::New(name, period, rise, fall, port))
}

//-------------------------------------------------------------------------------------------------

fn parse_port_delay(tokens: &[String], kind: DelayKind) -> Result<PortDelayConstraint, String>
{
    let mut clock_name = String::new();
    let mut delay = 0.0f64;
    let mut port_name = String::new();
    let mut delay_type = DelayType::Both;

    let mut i = 1usize;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-clock" => {
                i += 1;
                clock_name = tokens.get(i).ok_or("Missing clock name after -clock")?.clone();
            }
            "-max" => delay_type = DelayType::Max,
            "-min" => delay_type = DelayType::Min,
            _ => {
                if !tokens[i].starts_with('-') {
                    if let Ok(val) = tokens[i].parse::<f64>() {
                        delay = val;
                    } else {
                        port_name = tokens[i].clone();
                    }
                }
            }
        }
        i += 1;
    }
    Ok(PortDelayConstraint::New(kind, clock_name, delay, port_name, delay_type))
}

//-------------------------------------------------------------------------------------------------

fn parse_false_path(tokens: &[String]) -> Result<PathConstraint, String>
{
    let mut from = None;
    let mut to = None;
    let mut through = None;

    let mut i = 1usize;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-from" => {
                i += 1;
                from = tokens.get(i).cloned();
            }
            "-to" => {
                i += 1;
                to = tokens.get(i).cloned();
            }
            "-through" => {
                i += 1;
                through = tokens.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }
    Ok(PathConstraint::New(PathKind::FalsePath, 0, from, to, through))
}

//-------------------------------------------------------------------------------------------------

fn parse_multicycle_path(tokens: &[String]) -> Result<PathConstraint, String>
{
    let mut cycles = 2u32;
    let mut from = None;
    let mut to = None;

    let mut i = 1usize;
    if i < tokens.len() && !tokens[i].starts_with('-')
        && let Ok(c) = tokens[i].parse::<u32>()
    {
        cycles = c;
        i += 1;
    }
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-from" => {
                i += 1;
                from = tokens.get(i).cloned();
            }
            "-to" => {
                i += 1;
                to = tokens.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }
    Ok(PathConstraint::New(PathKind::MulticyclePath, cycles, from, to, None))
}

//-------------------------------------------------------------------------------------------------

/// Parses a complete SDC constraints script into structured `SdcConstraints`.
pub fn ParseSdc(sdc_text: &str) -> Result<SdcConstraints, String>
{
    let mut clocks = Stash::New();
    let mut delays = Stash::New();
    let mut paths = Stash::New();

    let preprocessed = preprocess_sdc_lines(sdc_text);
    for line in preprocessed {
        let tokens = tokenize_sdc_line(&line);
        if tokens.is_empty() {
            continue;
        }
        match tokens[0].as_str() {
            "create_clock" => {
                let clk = parse_create_clock(&tokens)?;
                clocks.Push(clk);
            }
            "set_input_delay" => {
                let delay = parse_port_delay(&tokens, DelayKind::Input)?;
                delays.Push(delay);
            }
            "set_output_delay" => {
                let delay = parse_port_delay(&tokens, DelayKind::Output)?;
                delays.Push(delay);
            }
            "set_false_path" => {
                let path = parse_false_path(&tokens)?;
                paths.Push(path);
            }
            "set_multicycle_path" => {
                let path = parse_multicycle_path(&tokens)?;
                paths.Push(path);
            }
            _ => {} // Ignore unsupported or vendor-specific commands gracefully
        }
    }

    Ok(SdcConstraints::New(clocks.IntoBuff(), delays.IntoBuff(), paths.IntoBuff()))
}

//-------------------------------------------------------------------------------------------------

fn preprocess_sdc_lines(sdc_text: &str) -> Vec<String>
{
    let mut combined = Vec::new();
    let mut current = String::new();

    for raw_line in sdc_text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(stripped) = trimmed.strip_suffix('\\') {
            current.push_str(stripped.trim());
            current.push(' ');
        } else {
            current.push_str(trimmed);
            combined.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        combined.push(current);
    }
    combined
}

//-------------------------------------------------------------------------------------------------
