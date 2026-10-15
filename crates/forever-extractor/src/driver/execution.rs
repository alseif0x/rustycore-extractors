// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: shared six-component production and completion lifecycle.
//! Platform setup admits and owns its input leases and collision storage.
//! This boundary preserves producer order, replay skips, verification positions
//! and package seals without choosing an operating system or acquiring paths.
use super::{Error, LocalStore, Path, Plan, Run, package};
use package::Component;

/// Adapter-owned capabilities survive every producer and final package seal.
/// Some input leases require an additional check before each active producer.
pub(super) trait Adapter {
    fn map_domain(&self) -> Option<&crate::map_domain::Domain> {
        None
    }
    fn extract_terrain(
        &self,
        plan: &Plan,
        store: &mut LocalStore,
        run: &mut Run,
    ) -> Result<(), Error> {
        super::maps(plan, store, run, self.map_domain())
    }
    fn verify_before_producer(&self) -> Result<(), Error> {
        Ok(())
    }
    fn verify_inputs(&self) -> Result<(), Error>;
    fn extract_collision(&self, store: &mut LocalStore, run: &mut Run) -> Result<(), Error>;
}

pub(super) fn execute(
    plan: &Plan,
    store: &mut LocalStore,
    output: &Path,
    run: Run,
    adapter: impl Adapter,
) -> Result<package::Summary, Error> {
    complete(
        plan.components,
        Production {
            plan,
            store,
            output,
            run,
            adapter,
        },
    )
}

fn replay_mask(plan: &Plan, completed: u8) -> u8 {
    if plan.is_available_terrain() {
        completed & !Component::Terrain.bit()
    } else {
        completed
    }
}

struct Production<'a, A> {
    plan: &'a Plan,
    store: &'a mut LocalStore,
    output: &'a Path,
    run: Run,
    adapter: A,
}

// Only the execution lifecycle is abstracted for bounded failure/order tests.
// Real producers retain their existing concrete stores, parsers and package.
trait Work {
    type Summary;
    fn completed_components(&self) -> Result<u8, Error>;
    fn verify_before_producer(&self) -> Result<(), Error>;
    fn db2(&mut self) -> Result<(), Error>;
    fn game_tables(&mut self) -> Result<(), Error>;
    fn cameras(&mut self) -> Result<(), Error>;
    fn terrain(&mut self) -> Result<(), Error>;
    fn collision(&mut self) -> Result<(), Error>;
    fn navigation(&mut self) -> Result<(), Error>;
    fn verify_inputs(&self) -> Result<(), Error>;
    fn finish_component(&mut self, component: Component) -> Result<(), Error>;
    fn finish(self) -> Result<Self::Summary, Error>;
}

impl<A: Adapter> Work for Production<'_, A> {
    type Summary = package::Summary;
    fn completed_components(&self) -> Result<u8, Error> {
        let completed = self.run.completed_components()?;
        // v12 replays even a sealed terrain stage to bind current coverage and outputs.
        Ok(replay_mask(self.plan, completed))
    }
    fn verify_before_producer(&self) -> Result<(), Error> {
        self.adapter.verify_before_producer()
    }
    fn db2(&mut self) -> Result<(), Error> {
        super::db2(
            self.plan,
            self.store,
            &mut self.run,
            self.adapter.map_domain(),
        )
    }
    fn game_tables(&mut self) -> Result<(), Error> {
        super::game_tables(self.plan, self.store, &mut self.run)
    }
    fn cameras(&mut self) -> Result<(), Error> {
        super::cameras(self.plan, self.store, &mut self.run)
    }
    fn terrain(&mut self) -> Result<(), Error> {
        self.adapter
            .extract_terrain(self.plan, self.store, &mut self.run)
    }
    fn collision(&mut self) -> Result<(), Error> {
        self.adapter.extract_collision(self.store, &mut self.run)
    }
    fn navigation(&mut self) -> Result<(), Error> {
        super::mmaps(
            self.plan,
            self.store,
            self.output,
            &mut self.run,
            self.adapter.map_domain(),
        )
    }
    fn verify_inputs(&self) -> Result<(), Error> {
        self.store
            .verify_unchanged()
            .map_err(|e| Error::from(e.code()))?;
        self.adapter.verify_inputs()
    }
    fn finish_component(&mut self, component: Component) -> Result<(), Error> {
        Ok(self.run.finish_component(component)?)
    }
    fn finish(self) -> Result<Self::Summary, Error> {
        Ok(self.run.finish()?)
    }
}

fn complete<W: Work>(selected: u8, mut work: W) -> Result<W::Summary, Error> {
    let completed = work.completed_components()?;
    for component in Component::ALL {
        if selected & component.bit() == 0 || completed & component.bit() != 0 {
            continue;
        }
        work.verify_before_producer()?;
        let result = match component {
            Component::Db2 => work.db2(),
            Component::GameTables => work.game_tables(),
            Component::Cameras => work.cameras(),
            Component::Terrain => work.terrain(),
            Component::Collision => work.collision(),
            Component::Navigation => work.navigation(),
        };
        result.map_err(|mut error| {
            error.component = Some(component);
            error
        })?;
        work.verify_inputs()?;
        // Completed producers include the validated empty-world case.
        work.finish_component(component)?;
    }
    work.verify_inputs()?;
    work.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Completed,
        Before,
        Produce(Component),
        CollisionAdapter,
        Verify,
        Seal(Component),
        Finish,
    }
    type Events = Rc<RefCell<Vec<Event>>>;

    struct Fake {
        events: Events,
        completed: u8,
        fail: Option<(Event, usize)>,
    }
    impl Fake {
        fn new(completed: u8, fail: Option<(Event, usize)>) -> (Self, Events) {
            let events = Events::default();
            (
                Self {
                    events: events.clone(),
                    completed,
                    fail,
                },
                events,
            )
        }
        fn event(&self, event: Event) -> Result<(), Error> {
            let mut events = self.events.borrow_mut();
            events.push(event);
            if self.fail.is_some_and(|(target, occurrence)| {
                target == event
                    && events.iter().filter(|&&item| item == event).count() == occurrence
            }) {
                return Err("authored_execution_failure".into());
            }
            Ok(())
        }
    }
    impl Work for Fake {
        type Summary = ();
        fn completed_components(&self) -> Result<u8, Error> {
            self.event(Event::Completed)?;
            Ok(self.completed)
        }
        fn verify_before_producer(&self) -> Result<(), Error> {
            self.event(Event::Before)
        }
        fn db2(&mut self) -> Result<(), Error> {
            self.event(Event::Produce(Component::Db2))
        }
        fn game_tables(&mut self) -> Result<(), Error> {
            self.event(Event::Produce(Component::GameTables))
        }
        fn cameras(&mut self) -> Result<(), Error> {
            self.event(Event::Produce(Component::Cameras))
        }
        fn terrain(&mut self) -> Result<(), Error> {
            self.event(Event::Produce(Component::Terrain))
        }
        fn collision(&mut self) -> Result<(), Error> {
            self.event(Event::CollisionAdapter)
        }
        fn navigation(&mut self) -> Result<(), Error> {
            self.event(Event::Produce(Component::Navigation))
        }
        fn verify_inputs(&self) -> Result<(), Error> {
            self.event(Event::Verify)
        }
        fn finish_component(&mut self, component: Component) -> Result<(), Error> {
            self.event(Event::Seal(component))
        }
        fn finish(self) -> Result<Self::Summary, Error> {
            self.event(Event::Finish)
        }
    }

    const ORDER: [Component; 6] = [
        Component::Db2,
        Component::GameTables,
        Component::Cameras,
        Component::Terrain,
        Component::Collision,
        Component::Navigation,
    ];
    fn all() -> u8 {
        ORDER.into_iter().fold(0, |mask, item| mask | item.bit())
    }
    fn expected(components: &[Component]) -> Vec<Event> {
        let mut events = vec![Event::Completed];
        for &component in components {
            events.extend([
                Event::Before,
                if component == Component::Collision {
                    Event::CollisionAdapter
                } else {
                    Event::Produce(component)
                },
                Event::Verify,
                Event::Seal(component),
            ]);
        }
        events.extend([Event::Verify, Event::Finish]);
        events
    }

    #[test]
    fn all_six_producers_keep_order_verification_and_seals() {
        let (work, events) = Fake::new(0, None);
        assert_eq!(complete(all(), work), Ok(()));
        assert_eq!(*events.borrow(), expected(&ORDER));
    }

    #[test]
    fn completed_and_unselected_stages_have_no_producer_or_checks() {
        let selected = all() & !Component::Cameras.bit();
        let completed = Component::Db2.bit() | Component::Collision.bit();
        let (work, events) = Fake::new(completed, None);
        assert_eq!(complete(selected, work), Ok(()));
        assert_eq!(
            *events.borrow(),
            expected(&[
                Component::GameTables,
                Component::Terrain,
                Component::Navigation
            ])
        );
    }

    #[test]
    fn entirely_completed_resume_still_verifies_before_final_seal() {
        let (work, events) = Fake::new(all(), None);
        assert_eq!(complete(all(), work), Ok(()));
        assert_eq!(*events.borrow(), expected(&[]));
    }

    #[test]
    fn collision_dispatches_only_through_its_adapter_stage() {
        let (work, events) = Fake::new(0, None);
        assert_eq!(complete(Component::Collision.bit(), work), Ok(()));
        assert_eq!(*events.borrow(), expected(&[Component::Collision]));
    }

    #[test]
    fn completion_read_failure_has_no_checks_producers_or_seals() {
        let (work, events) = Fake::new(0, Some((Event::Completed, 1)));
        let error = complete(all(), work).unwrap_err();
        assert_eq!(error.component, None);
        assert_eq!(*events.borrow(), [Event::Completed]);
    }

    #[test]
    fn pre_producer_verification_failure_prevents_production_and_sealing() {
        let (work, events) = Fake::new(0, Some((Event::Before, 1)));
        let error = complete(all(), work).unwrap_err();
        assert_eq!(error.component, None);
        assert_eq!(*events.borrow(), [Event::Completed, Event::Before]);
    }

    #[test]
    fn every_producer_failure_tags_its_component_and_prevents_its_seal() {
        for (index, component) in ORDER.into_iter().enumerate() {
            let mut prefix = expected(&ORDER[..=index]);
            prefix.truncate(prefix.len() - 4); // failed producer is the last event
            let failed = *prefix.last().unwrap();
            let (work, events) = Fake::new(0, Some((failed, 1)));
            let error = complete(all(), work).unwrap_err();
            assert_eq!(error.component, Some(component));
            assert_eq!(*events.borrow(), prefix);
        }
    }

    #[test]
    fn post_producer_verification_failure_prevents_component_and_final_seals() {
        let (work, events) = Fake::new(0, Some((Event::Verify, 1)));
        let error = complete(all(), work).unwrap_err();
        assert_eq!(error.component, None);
        assert_eq!(
            *events.borrow(),
            [
                Event::Completed,
                Event::Before,
                Event::Produce(Component::Db2),
                Event::Verify
            ]
        );
    }

    #[test]
    fn final_verification_failure_prevents_package_seal() {
        let (work, events) = Fake::new(0, Some((Event::Verify, 7)));
        let error = complete(all(), work).unwrap_err();
        assert_eq!(error.component, None);
        let mut prefix = expected(&ORDER);
        prefix.pop();
        assert_eq!(*events.borrow(), prefix);
    }

    #[test]
    fn component_seal_failure_prevents_later_producers_and_package_seal() {
        let (work, events) = Fake::new(0, Some((Event::Seal(Component::Db2), 1)));
        let error = complete(all(), work).unwrap_err();
        assert_eq!(error.component, None);
        let mut prefix = expected(&ORDER[..1]);
        prefix.truncate(prefix.len() - 2);
        assert_eq!(*events.borrow(), prefix);
    }

    #[test]
    fn final_seal_error_is_returned_without_component_relabeling() {
        let (work, events) = Fake::new(all(), Some((Event::Finish, 1)));
        let error = complete(all(), work).unwrap_err();
        assert_eq!(error.code, "authored_execution_failure");
        assert_eq!(error.component, None);
        assert_eq!(*events.borrow(), expected(&[]));
    }
}

#[cfg(test)]
mod v12_replay_tests {
    use super::*;
    #[test]
    fn v12_replays_sealed_terrain_and_older_profiles_keep_skips() {
        let old = Plan::parse(include_str!(
            "../../../../tests/contracts/fixtures/forever-product-v11/disabled.tsv"
        ))
        .unwrap();
        assert_eq!(replay_mask(&old, 63), 63);
        let selected = Plan {
            publication_profile: crate::plan::PublicationProfile::Build70374AvailableTerrainV1,
            components: Component::Terrain.bit(),
            db2: Vec::new(),
            terrain: crate::plan::Selection::All,
            ..old
        };
        selected.validate().unwrap();
        assert_eq!(replay_mask(&selected, 8), 0);
        assert_eq!(replay_mask(&selected, 63), 55);
        let compatible = Plan {
            publication_profile: crate::plan::PublicationProfile::Build70374CompatibleTerrainV1,
            ..selected
        };
        compatible.validate().unwrap();
        assert_eq!(replay_mask(&compatible, 8), 0);
        assert_eq!(replay_mask(&compatible, 63), 55);
    }
}
