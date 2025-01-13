use std::collections::HashMap;

pub struct TimerEntry{
    start: std::time::Instant,
    end: Option<std::time::Instant>,
}

impl TimerEntry{
    pub fn new() -> Self{
        Self{
            start: std::time::Instant::now(),
            end: None,
        }
    }
    pub fn start(&mut self){
        self.start = std::time::Instant::now();
    }
    pub fn stop(&mut self){
        self.end = Some(std::time::Instant::now());
    }
    
    pub fn duration(&self) -> std::time::Duration{
        match self.end{
            Some(end) => end.duration_since(self.start),
            None => std::time::Instant::now().duration_since(self.start),
        }
    }
}
pub struct DebugManager{
    entries: HashMap<String, TimerEntry> // consider using Mutex in future for a multithreaded environment
}

impl DebugManager{
    pub async fn new() -> Self{
        Self{
            entries: HashMap::new()
        }
    }

    pub fn start_timer(&mut self, label: &str){
        self.entries.insert(label.to_string(), TimerEntry::new());
    }
    pub fn stop_timer(&mut self, label: &str) -> Option<TimerEntry>{
        self.entries.remove(label)
    }
}