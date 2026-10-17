// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: Windows resident product dispatch and supplied-key leases.
//! Complete existing producers share the Windows package owner. Supplied keys
//! remain leased until completion; no disk-storage or path-only identity fallback.
use super::*;
use forever_platform::windows::ReadFile;

pub(super) fn verify_keys(source: Option<&ReadFile>) -> Result<(), Error> {
    if let Some(source) = source {
        source
            .verify_unchanged()
            .map_err(|_| Error::from("product_keys_input_changed"))?;
    }
    Ok(())
}

pub(super) fn inspect(
    plan: &Plan,
    client: &Path,
    producer: [u8; 32],
    keys: Option<crate::input::SuppliedKeys>,
) -> Result<Context, Error> {
    let (keyring, source) = match keys {
        Some(keys) => {
            let (keyring, _, source) = keys.into_windows_parts();
            (Some(keyring), Some(source))
        }
        None => (None, None),
    };
    verify_keys(source.as_ref())?;
    let result =
        open_inner(plan, client, keyring).and_then(|store| context(plan, &store, producer));
    // The source lease survives context admission and is rechecked even on
    // client failure. Key names/material/identity never enter the context.
    verify_keys(source.as_ref())?;
    result
}

pub(super) fn extract_inner(
    plan: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
    keys: Option<crate::input::SuppliedKeys>,
    scratch: Option<&Path>,
) -> Result<package::Summary, Error> {
    admit(plan)?;
    if scratch.is_some() {
        return Err("product_scratch_forbidden".into());
    }
    validate_keys(plan, keys.is_some())?;
    let (keyring, key_source) = match keys {
        Some(keys) => {
            let (keyring, _, source) = keys.into_windows_parts();
            (Some(keyring), Some(source))
        }
        None => (None, None),
    };
    verify_keys(key_source.as_ref())?;
    let result = (|| {
        let mut store = open_inner(plan, client, keyring)?;
        let context = context(plan, &store, producer)?;
        let data = client.join("Data");
        let archives = data.join("data");
        let configs = data.join("config");
        let protected = [
            client,
            data.as_path(),
            archives.as_path(),
            configs.as_path(),
        ];
        let make_run = |sources: &[&ReadFile]| {
            if resume {
                Run::resume_with_sources(output, context, &protected, Default::default(), sources)
            } else {
                Run::create_with_sources(output, context, &protected, Default::default(), sources)
            }
        };
        let run = match key_source.as_ref() {
            Some(source) => make_run(&[source]),
            None => make_run(&[]),
        }?;
        execution::execute(
            plan,
            &mut store,
            output,
            run,
            WindowsExecution {
                key_source: key_source.as_ref(),
            },
        )
    })();
    // Keep the lease alive through finish and check it on every return path.
    verify_keys(key_source.as_ref())?;
    result
}

/// Windows owns the supplied-key read lease for the complete resident run.
struct WindowsExecution<'a> {
    key_source: Option<&'a ReadFile>,
}

impl execution::Adapter for WindowsExecution<'_> {
    fn verify_before_producer(&self) -> Result<(), Error> {
        verify_keys(self.key_source)
    }

    fn verify_inputs(&self) -> Result<(), Error> {
        verify_keys(self.key_source)
    }

    fn extract_collision(
        &self,
        _plan: &Plan,
        store: &mut LocalStore,
        run: &mut Run,
    ) -> Result<(), Error> {
        vmaps(store, run)
    }
}
