//
// Vislumino - Astronomy Visualization Tools
// Copyright (c) 2022 Filip Szczerek <ga.software@yahoo.com>
//
// This file is part of Vislumino.
//
// Vislumino is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// Vislumino is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Vislumino.  If not, see <http://www.gnu.org/licenses/>.
//

use std::cell::RefCell;
use std::rc::Weak;

pub trait Subscriber<T> {
    fn notify(&mut self, value: &T);
}

pub struct SubscriberCollection<T> {
    subscribers: Vec<Weak<RefCell<dyn Subscriber<T>>>>
}

// not using #[derive(Default)], as it (needlessly) imposes `Default` also on `T`
impl<T> Default for SubscriberCollection<T> {
    fn default() -> SubscriberCollection<T> {
        SubscriberCollection{ subscribers: vec![] }
    }
}

impl<T> SubscriberCollection<T> {
    pub fn new() -> SubscriberCollection<T> {
        SubscriberCollection{ subscribers: vec![] }
    }

    /// Notifies all still existing subscribers; removes those no longer available.
    pub fn notify(&mut self, value: &T) {
        self.subscribers.retain_mut(|subscriber| {
            match subscriber.upgrade() {
                Some(subscriber) => {
                    subscriber.borrow_mut().notify(value);
                    true
                },

                None => false
            }
        });
    }

    pub fn add(&mut self, subscriber: Weak<RefCell<dyn Subscriber<T>>>) {
        self.subscribers.push(subscriber);
    }
}
