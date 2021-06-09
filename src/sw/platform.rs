//! Multi-platform support interfaces.

use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::application::Application;
use super::error::Error;
use super::instrument::Spec;
use super::Platform;
use super::PlatformSupport;
use super::Result;

/// Testbed support for the Tock OS platform.
#[derive(Clone, Debug)]
pub struct Tock {
    tockloader_path: PathBuf,
    // The use of this type (and the RefCell) to wrap this type is in lieu of
    // doing something more robust such as querying the device itself for its
    // software.
    loaded_apps: RefCell<HashSet<String>>,
    source_path: PathBuf,
}

impl Tock {
    /// Create a new Tock platform instance.
    pub fn new(tockloader_path: &Path, source_path: &Path) -> Tock {
        Tock {
            tockloader_path: tockloader_path.to_path_buf(),
            loaded_apps: RefCell::new(HashSet::new()),
            source_path: source_path.to_path_buf(),
        }
    }

    /// Retrieve a `make` command.
    fn make_command(&self) -> Command {
        // NOTICE: forcing use of the Hail board configuration.
        let make_work_dir = self.source_path.clone()
            .join("boards/hail");

        // Assuming make is in /usr/bin.
        let mut command = Command::new("/usr/bin/make");
        command
            .args(&["-C", make_work_dir.to_str().unwrap()])
            .envs(env::vars());

        command
    }

    /// Build Tock OS.
    #[allow(dead_code)]
    fn build(&self) -> Result<Output> {

        println!("Building Tock OS.");
        self.make_command()
            .output()
            .map_err(|io_err| Error::IO(io_err))
    }


    /// Build Tock OS according to a spec.
    fn build_instrumented(&self, spec: &Spec) -> Result<Output> {
        // TODO: centralize and 'uniquify' this path.
        let spec_path = Path::new("/var/tmp/__autogen_trace.json");
        spec.write(spec_path)?;

        println!("Building instrumented Tock OS.");
        self.make_command()
            .envs(vec![("TRACE_SPEC_PATH".to_string(), spec_path.to_str().unwrap().to_string()),
                       ("TRACE_VERBOSE".to_string(), "1".to_string())])
            .output()
            .map_err(|io_err| Error::IO(io_err))
    }

    fn program(&self) -> Result<Output> {
        // NOTICE: forcing use of the Hail board configuration.
        let make_work_dir = self.source_path.clone()
            .join("boards/hail");

        println!("Programming target with Tock OS from '{}'.", make_work_dir.display());
        self.make_command()
            .args(&["program"])
            .output()
            .map_err(|io_err| Error::IO(io_err))
    }
}

impl PlatformSupport for Tock {
    fn platform(&self) -> Platform {
        Platform::Tock
    }

    fn load(&self, app: &Application) -> Result<()> {
        let tockloader_path_str = self.tockloader_path.to_str()
            .ok_or(Error::Other(format!("cannot convert '{}' to Unicode", self.tockloader_path.display())))?;
        let path = app.get_for(self.platform())?;
        let app_path_str = path.to_str()
            .ok_or(Error::Other(format!("cannot convert '{}' to Unicode", path.display())))?;

        let output = Command::new(tockloader_path_str)
            .args(&["install", app_path_str])
            .output()?;

        if output.status.success() {
            self.loaded_apps.borrow_mut()
                .insert(app.get_id().to_string());
            Ok(())
        } else {
            Err(Error::Tool(output))
        }
    }

    fn unload(&self, app_id: &str) -> Result<()> {
        // No need to remove what's not there.
        let was_present =  self.loaded_apps.borrow_mut()
            .remove(app_id);
        if !was_present {
            Ok(())
        } else {
            let tockloader_path_str = self.tockloader_path.to_str()
                .ok_or(Error::Other(format!("cannot convert '{}' to Unicode", self.tockloader_path.display())))?;

            let output = Command::new(tockloader_path_str)
                .args(&["uninstall"])
                .output()?;

            if output.status.success() {
                Ok(())
            } else {
                // Question: what state is the device in if we fail?
                Err(Error::Tool(output))
            }
        }
    }

    fn loaded_software(&self) -> HashSet<String> {
        self.loaded_apps.borrow().iter()
            .cloned()
            .collect()
    }

    fn reconfigure(&self, trace_points: &Vec<String>) -> Result<Spec> {
        let spec = Spec::new(trace_points.iter().map(|s| s.as_ref()));
        let output = if trace_points.is_empty() {
            self.build()?
        } else {
            self.build_instrumented(&spec)?
        };

        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout.clone())
                .unwrap_or("<<Could not process stdout output.>>".to_string());
            let stderr = String::from_utf8(output.stderr.clone())
                .unwrap_or("<<Could not process stderr output.>>".to_string());
            println!("Build failed.\nSTDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);
            Err(Error::Tool(output))
        } else {
            self.program()?;
            Ok(spec)
        }
    }
}
