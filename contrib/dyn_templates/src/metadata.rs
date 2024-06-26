use std::fmt;
use std::borrow::Cow;

use rocket::{Request, Rocket, Ignite, Sentinel};
use rocket::http::{Status, ContentType};
use rocket::request::{self, FromRequest};
use rocket::serde::Serialize;

use crate::{Template, context::ContextManager};

/// Request guard for dynamically querying template metadata.
///
/// # Usage
///
/// The `Metadata` type implements Rocket's [`FromRequest`] trait, so it can be
/// used as a request guard in any request handler.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// # #[macro_use] extern crate rocket_dyn_templates;
/// use rocket_dyn_templates::{Template, Metadata, context};
///
/// #[get("/")]
/// fn homepage(metadata: Metadata) -> Template {
///     // Conditionally render a template if it's available.
///     # let context = ();
///     if metadata.contains_template("some-template") {
///         Template::render("some-template", &context)
///     } else {
///         Template::render("fallback", &context)
///     }
/// }
///
/// fn main() {
///     rocket::build()
///         .attach(Template::fairing())
///         // ...
///     # ;
/// }
/// ```
pub struct Metadata<'a>(&'a ContextManager);

impl Metadata<'_> {
    /// Returns `true` if the template with the given `name` is currently
    /// loaded.  Otherwise, returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// # extern crate rocket_dyn_templates;
    /// #
    /// use rocket_dyn_templates::Metadata;
    ///
    /// #[get("/")]
    /// fn handler(metadata: Metadata) {
    ///     // Returns `true` if the template with name `"name"` was loaded.
    ///     let loaded = metadata.contains_template("name");
    /// }
    /// ```
    pub fn contains_template(&self, name: &str) -> bool {
        self.0.context().templates.contains_key(name)
    }

    /// Returns `true` if template reloading is enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// # extern crate rocket_dyn_templates;
    /// #
    /// use rocket_dyn_templates::Metadata;
    ///
    /// #[get("/")]
    /// fn handler(metadata: Metadata) {
    ///     // Returns `true` if template reloading is enabled.
    ///     let reloading = metadata.reloading();
    /// }
    /// ```
    pub fn reloading(&self) -> bool {
        self.0.is_reloading()
    }

    /// Directly render the template named `name` with the context `context`
    /// into a `String`. Also returns the template's detected `ContentType`. See
    /// [`Template::render()`] for more details on rendering.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::ContentType;
    /// use rocket_dyn_templates::{Metadata, Template, context};
    ///
    /// #[get("/")]
    /// fn send_email(metadata: Metadata) -> Option<()> {
    ///     let (mime, string) = metadata.render("email", context! {
    ///         field: "Hello, world!"
    ///     })?;
    ///
    ///     # /*
    ///     send_email(mime, string).await?;
    ///     # */
    ///     Some(())
    /// }
    ///
    /// #[get("/")]
    /// fn raw_render(metadata: Metadata) -> Option<(ContentType, String)> {
    ///     metadata.render("index", context! { field: "Hello, world!" })
    /// }
    ///
    /// // Prefer the following, however, which is nearly identical but pithier:
    ///
    /// #[get("/")]
    /// fn render() -> Template {
    ///     Template::render("index", context! { field: "Hello, world!" })
    /// }
    /// ```
    pub fn render<S, C>(&self, name: S, context: C) -> Option<(ContentType, String)>
        where S: Into<Cow<'static, str>>, C: Serialize
    {
        Template::render(name.into(), context).finalize(&self.0.context()).ok()
    }
}

impl fmt::Debug for Metadata<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(&self.0.context().templates)
            .finish()
    }
}

impl Sentinel for Metadata<'_> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        if rocket.state::<ContextManager>().is_none() {
            error!(
                "uninitialized template context: missing `Template::fairing()`.\n\
                To use templates, you must attach `Template::fairing()`."
            );

            return true;
        }

        false
    }
}

/// Retrieves the template metadata. If a template fairing hasn't been attached,
/// an error is printed and an empty `Err` with status `InternalServerError`
/// (`500`) is returned.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Metadata<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        request.rocket().state::<ContextManager>()
            .map(|cm| request::Outcome::Success(Metadata(cm)))
            .unwrap_or_else(|| {
                error!(
                    "uninitialized template context: missing `Template::fairing()`.\n\
                    To use templates, you must attach `Template::fairing()`."
                );

                request::Outcome::Error((Status::InternalServerError, ()))
            })
    }
}
