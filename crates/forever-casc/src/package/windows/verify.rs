// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: bounded same-lease Windows package integrity inspection.
use super::*;
pub fn verify(output: &Path, expected: Option<Context>, limits: Limits) -> Result<Summary, Error> {
    verify_inner(output, expected, limits, None)
}
/// Callbacks receive only a bounded reader of the same lease that was hashed.
/// Their results remain provisional until final namespace/state rechecks pass.
pub fn verify_with(
    output: &Path,
    expected: Option<Context>,
    limits: Limits,
    mut inspect: impl FnMut(&Artifact, &mut dyn Read) -> Result<(), Error>,
) -> Result<Summary, Error> {
    verify_inner(output, expected, limits, Some(&mut inspect))
}
type Inspector<'a> = &'a mut dyn FnMut(&Artifact, &mut dyn Read) -> Result<(), Error>;
fn verify_inner(
    output: &Path,
    expected: Option<Context>,
    limits: Limits,
    inspect: Option<Inspector<'_>>,
) -> Result<Summary, Error> {
    limits.validate()?;
    let mut root = Root::open(output, native_limits(limits))?;
    let _lock = root.lock(false)?;
    let entries = observe(&root)?;
    if entry(&entries, INCOMPLETE).is_some() {
        return Err(Error::Incomplete);
    }
    let context = context(&root)?;
    if expected.is_some_and(|v| v != context) {
        return Err(Error::Context);
    }
    state(&root, context)?;
    let meta = entry(&entries, MANIFEST).ok_or(Error::Incomplete)?;
    if meta.kind != Kind::File {
        return Err(Error::Path);
    }
    if meta.identity.size > limits.manifest_bytes {
        return Err(Error::Limit);
    }
    let bounded = inventory_limits(&root, limits, &entries, 0)?;
    let (_, inventory) = root.with_read_file(MANIFEST, Some(meta.identity), |file| {
        let mut r = io::BufReader::new(&mut *file);
        if line(&mut r)? != header(context).as_bytes() {
            return Err(Error::Schema);
        }
        let mut inventory = Inventory::new(bounded);
        loop {
            let b = line(&mut r)?;
            if b.starts_with(b"]") {
                if b != footer(inventory.summary(context)).as_bytes() {
                    return Err(Error::Inventory);
                }
                let mut end = [0];
                if r.read(&mut end).map_err(|_| Error::Io)? != 0 {
                    return Err(Error::Schema);
                }
                break;
            }
            let s = std::str::from_utf8(&b)
                .map_err(|_| Error::Schema)?
                .strip_suffix('\n')
                .ok_or(Error::Schema)?;
            let a = Artifact::parse(s.strip_suffix(',').unwrap_or(s), context, bounded)?;
            if inventory
                .facts
                .last()
                .is_some_and(|p| !model::ordered(&p.path, &a.path))
            {
                return Err(Error::Inventory);
            }
            inventory.push(a)?;
        }
        drop(r);
        let mf = manifest_fact(context, &inventory)?;
        if mf.bytes != meta.identity.size {
            return Err(Error::Schema);
        }
        file.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
        if crate::integrity::reader(file, mf.bytes).map_err(|_| Error::Digest)? != mf.sha256 {
            return Err(Error::Schema);
        }
        Ok(inventory)
    })?;
    for a in &inventory.facts {
        let name = receipt(a);
        let e = entry(&entries, &name).ok_or(Error::Inventory)?;
        if small(&root, &name, Some(e.identity))? != a.json().as_bytes() {
            return Err(Error::Inventory);
        }
    }
    if seals(&root, &entries, context, &inventory)? != context.components {
        return Err(Error::Incomplete);
    }
    validate_facts(&root, &entries, &inventory)?;
    closed(
        &entries,
        context,
        &inventory,
        false,
        true,
        &[],
        (inventory.facts.len(), inventory.facts.len()),
    )?;
    root.verify_membership_charged(&entries, inventory.charged_bytes())?;
    if let Some(inspect) = inspect {
        for a in &inventory.facts {
            let e = entry(&entries, &a.path).ok_or(Error::Inventory)?;
            root.with_read_file(&a.path, Some(e.identity), |file| {
                if crate::integrity::reader(&mut *file, a.bytes).map_err(|_| Error::Digest)?
                    != a.sha256
                {
                    return Err(Error::Digest);
                }
                file.seek(SeekFrom::Start(0)).map_err(|_| Error::Io)?;
                inspect(a, file)
            })?;
        }
        for a in &inventory.facts {
            let name = receipt(a);
            let e = entry(&entries, &name).ok_or(Error::Inventory)?;
            if small(&root, &name, Some(e.identity))? != a.json().as_bytes() {
                return Err(Error::Inventory);
            }
        }
        if seals(&root, &entries, context, &inventory)? != context.components {
            return Err(Error::Incomplete);
        }
        hash(
            &root,
            &manifest_fact(context, &inventory)?,
            Some(meta.identity),
        )?;
        root.verify_membership_charged(&entries, inventory.charged_bytes())?;
    }
    state(&root, context)?;
    Ok(inventory.summary(context))
}
