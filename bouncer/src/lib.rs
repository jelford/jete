use std::time::{Duration, Instant};

const ZERO_DURATION: Duration = Duration::from_millis(0);

pub struct Bouncer {
    deadline: Option<Instant>,
    millis_budget: u128,
    start_time: Instant,
    hot_deadline_proximity: Option<u128>,
}

pub struct BouncerBuilder {
    time_between_deadlines: Duration,
    grace_period: Option<Duration>,
}

impl BouncerBuilder {
    pub fn time_between_deadlines(mut self, duration: Duration) -> BouncerBuilder {
        self.time_between_deadlines = duration;
        self
    }

    pub fn skip_hot_deadline(mut self, duration: Duration) -> BouncerBuilder {
        self.grace_period = Some(duration);
        self
    }

    pub fn build(self) -> Bouncer {
        Bouncer {
            millis_budget: self.time_between_deadlines.as_millis(),
            hot_deadline_proximity: self.grace_period.map(|gp| gp.as_millis()),
            start_time: Instant::now(),
            deadline: None,
        }
    }
}

impl Bouncer {
    pub fn builder() -> BouncerBuilder {
        BouncerBuilder {
            time_between_deadlines: ZERO_DURATION,
            grace_period: None,
        }
    }

    pub fn mark(&mut self) {
        if self.deadline.is_some() {
            return;
        }

        if self.millis_budget == 0 {
            self.deadline = Some(Instant::now() - Duration::from_nanos(1));
            return;
        }

        let now = Instant::now();
        let millis_in = now
            .checked_duration_since(self.start_time)
            .map(|d| d.as_millis() % self.millis_budget)
            .unwrap_or(0);

        let mut time_until_next_deadline = self.millis_budget - millis_in;
        if let Some(grace_period) = self.hot_deadline_proximity {
            if time_until_next_deadline < grace_period {
                time_until_next_deadline = time_until_next_deadline + self.millis_budget;
            }
        }

        let next_deadline: Instant = now
            .checked_add(Duration::from_millis(
                time_until_next_deadline.min(u64::max_value() as u128) as u64,
            ))
            .expect("We have reached the end of time.");

        self.deadline = Some(next_deadline);
    }

    pub fn current_deadline(&self) -> &Option<Instant> {
        &self.deadline
    }

    pub fn duration_until_deadline(&self) -> Option<Duration> {
        let now = Instant::now();
        self.deadline.map(|deadline| {
            deadline
                .checked_duration_since(now)
                .unwrap_or(ZERO_DURATION)
        })
    }

    pub fn clear(&mut self) {
        self.deadline = None;
    }

    pub fn expired(&self) -> bool {
        self.deadline.map(|d| d <= Instant::now()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bouncer_expires_immediately() {
        let mut d = Bouncer::builder().build();
        d.mark();
        assert!(d.expired());
    }

    #[test]
    fn bouncer_with_long_deadline_is_now_expired_immediately() {
        let mut d = Bouncer::builder()
            .time_between_deadlines(Duration::from_secs(10))
            .build();
        d.mark();
        assert!(!d.expired());
    }

    #[test]
    fn bouncer_gives_reasonable_time_to_expiry_after_marking() {
        let mut d = Bouncer::builder()
            .time_between_deadlines(Duration::from_secs(10))
            .build();
        d.mark();
        let time_to_expiry = d.duration_until_deadline().unwrap();
        assert!(time_to_expiry > Duration::from_millis(9_500));
        assert!(time_to_expiry < Duration::from_secs(10));
    }

    #[test]
    fn mark_near_hot_deadline_skips_to_next() {
        let time_between_deadlines = Duration::from_millis(10);
        let mut hot_deadline_skipper = Bouncer::builder()
            .time_between_deadlines(time_between_deadlines)
            .skip_hot_deadline(time_between_deadlines - Duration::from_micros(1))
            .build();
        let mut bouncer_sticking_to_deadlines = Bouncer::builder()
            .time_between_deadlines(time_between_deadlines)
            .build();

        let start_of_poll = Instant::now();
        let mut saw_deadline_jump_ahead = false;

        while match Instant::now().checked_duration_since(start_of_poll) {
            None => true,
            Some(d) => d < Duration::from_secs(1),
        } {
            std::thread::sleep(Duration::from_micros(1));

            hot_deadline_skipper.mark();
            bouncer_sticking_to_deadlines.mark();

            if hot_deadline_skipper.duration_until_deadline().unwrap() > time_between_deadlines {
                saw_deadline_jump_ahead = true;
                break;
            }

            assert!(
                bouncer_sticking_to_deadlines
                    .duration_until_deadline()
                    .unwrap()
                    <= time_between_deadlines
            );

            hot_deadline_skipper.clear();
            bouncer_sticking_to_deadlines.clear();
        }

        assert!(saw_deadline_jump_ahead);
    }
}
