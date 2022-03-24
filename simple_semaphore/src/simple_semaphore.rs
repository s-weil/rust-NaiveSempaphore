#![allow(dead_code)]
use std::sync::{Condvar, Mutex};

struct NaiveSemaphore {
    current: Mutex<usize>,
    max: usize,
    is_locked: Mutex<bool>,
    waiter: Condvar,
}

impl NaiveSemaphore {
    pub fn new(max: usize) -> Self {
        Self {
            max,
            current: Mutex::new(0),
            is_locked: Mutex::new(false),
            waiter: Condvar::new(),
        }
    }

    /// The count of currently running threads.
    pub fn current_count(&self) -> usize {
        self.current.lock().unwrap().clone()
    }

    /// Release a waiting thread, reduce the current count.
    pub fn release_one(&self) {
        let mut current = self.current.lock().unwrap();
        if *current > 0 {
            let mut is_locked = self.is_locked.lock().unwrap();
            if *is_locked {
                *is_locked = false;
                self.waiter.notify_one(); // wake up one waiting thread
            }
            *current -= 1;
        }
    }

    /// Block a thread in case the current count exceeds 'max'.
    pub fn wait(&self) {
        let mut locked = self.is_locked.lock().unwrap();

        if *locked {
            locked = self.waiter.wait(locked).unwrap();
        }
        drop(locked);

        let mut current = self.current.lock().unwrap();
        *current += 1;

        if *current >= self.max {
            let mut locked = self.is_locked.lock().unwrap();
            *locked = true;
        }
        drop(current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use stopwatch::Stopwatch;

    fn aggregate_counts<'a>(
        hm: &'a mut HashMap<char, usize>,
        update_hm: &HashMap<char, usize>,
    ) -> &'a HashMap<char, usize> {
        for (c, ct) in update_hm.iter() {
            if let Some(current_ct) = hm.get_mut(&c) {
                *current_ct += ct;
            } else {
                hm.insert(*c, *ct);
            }
        }
        hm
    }

    fn word_frequency(word: &str) -> HashMap<char, usize> {
        let mut hm = HashMap::new();
        for c in word.chars() {
            if let Some(ct) = hm.get_mut(&c) {
                *ct += 1;
            } else {
                hm.insert(c, 1);
            }
        }
        hm
    }

    // TODO: replace println's by async logger!
    // TODO: check with spans and jaeger?
    fn frequency(input: &'static [&str], worker_count: usize) -> usize {
        let naive_semaphore = Arc::new(NaiveSemaphore::new(worker_count));
        let mut handles = vec![];

        let results: HashMap<char, usize> = HashMap::new();
        let sharedstate_res = Arc::new(Mutex::new(results));

        for (idx, word) in input.iter().enumerate() {
            let semaphore = Arc::clone(&naive_semaphore);
            let ss_res = Arc::clone(&sharedstate_res);

            let handle = std::thread::spawn(move || {
                let sw = Stopwatch::start_new();
                println!("thread {} - cc {}: waiting", idx, semaphore.current_count());

                semaphore.wait();

                println!(
                    "thread {} - cc {} - {} ms: starting work",
                    idx,
                    semaphore.current_count(),
                    sw.elapsed_ms()
                );

                // just as a test to simulate a heavier workload
                std::thread::sleep(std::time::Duration::from_secs(1));
                let word_count = word_frequency(word);

                let res_lock = &mut ss_res.lock().unwrap();
                aggregate_counts(res_lock, &word_count);
                drop(res_lock);

                println!(
                    "thread {} - cc {} - {} ms: work done",
                    idx,
                    semaphore.current_count(),
                    sw.elapsed_ms()
                );
                semaphore.release_one();
                println!(
                    "thread {} - cc {} - {} ms: released",
                    idx,
                    semaphore.current_count(),
                    sw.elapsed_ms()
                );
            });

            handles.push(handle);
        }

        let num_threads = handles.len();

        for handle in handles {
            handle.join().unwrap();
        }

        dbg!(&sharedstate_res.lock().unwrap());
        num_threads
    }

    // Poem by Friedrich Schiller. The corresponding music is the European Anthem.
    const ODE_AN_DIE_FREUDE: [&str; 32] = [
        "Freude schöner Götterfunken",
        "Tochter aus Elysium,",
        "Wir betreten feuertrunken,",
        "Himmlische, dein Heiligtum!",
        "Deine Zauber binden wieder",
        "Was die Mode streng geteilt;",
        "Alle Menschen werden Brüder,",
        "Wo dein sanfter Flügel weilt.",
        "Freude schöner Götterfunken",
        "Tochter aus Elysium,",
        "Wir betreten feuertrunken,",
        "Himmlische, dein Heiligtum!",
        "Deine Zauber binden wieder",
        "Was die Mode streng geteilt;",
        "Alle Menschen werden Brüder,",
        "Wo dein sanfter Flügel weilt.",
        "Freude schöner Götterfunken",
        "Tochter aus Elysium,",
        "Wir betreten feuertrunken,",
        "Himmlische, dein Heiligtum!",
        "Deine Zauber binden wieder",
        "Was die Mode streng geteilt;",
        "Alle Menschen werden Brüder,",
        "Wo dein sanfter Flügel weilt.",
        "Freude schöner Götterfunken",
        "Tochter aus Elysium,",
        "Wir betreten feuertrunken,",
        "Himmlische, dein Heiligtum!",
        "Deine Zauber binden wieder",
        "Was die Mode streng geteilt;",
        "Alle Menschen werden Brüder,",
        "Wo dein sanfter Flügel weilt.",
    ];

    #[test]
    // #[ignore]
    fn test_freq_count() {
        let mut sw = stopwatch::Stopwatch::start_new();

        let num_done = frequency(&ODE_AN_DIE_FREUDE, 4);

        sw.stop();
        println!("done within {} ms", sw.elapsed_ms());
        assert_eq!(num_done, 32);
    }
}
