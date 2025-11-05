// Modules
#[path = "src/build_constants.rs"]
mod build_constants;

// Uses
use std::io::Result;

#[cfg(windows)]
use winresource::WindowsResource;

#[cfg(windows)]
use self::build_constants::ICON_WINDOWS_ID;

// Constants
#[cfg(windows)]
const BUILD_ASSETS_DIR: &str = ".";
#[cfg(windows)]
const NEUTRAL_LCID: u16 = 0x0000;

/// Build script that prepares the application.
fn main() -> Result<()> {
	// OS-Specific Executable Packaging
	executable_packaging()?;

	Ok(())
}

/// Sets up executable manifests, icons, etc. OS-Specific.
fn executable_packaging() -> Result<()> {
	#[cfg(windows)]
	{
		WindowsResource::new()
			.set_icon_with_id(
				format!("{}/{}", BUILD_ASSETS_DIR, "logo.ico").as_str(),
				ICON_WINDOWS_ID.to_string().as_str(),
			)
			.set_language(NEUTRAL_LCID)
			.compile()?;
	}

	Ok(())
}
