//! SCIP symbol string construction for PHP symbols.
//!
//! Generates fully-qualified SCIP symbol strings in the format:
//! `scip-php composer <package> <version> <descriptor>`
//!
//! PHP namespace separators (`\`) are replaced with `/` in descriptors.

use std::path::Path;

use crate::composer::Composer;

/// Package identity for SCIP symbol generation.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

impl PackageInfo {
    /// Create a new PackageInfo with the given name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        PackageInfo {
            name: name.into(),
            version: version.into(),
        }
    }

    /// Create a PackageInfo for the current project from a SymbolNamer.
    pub fn project(namer: &SymbolNamer) -> Self {
        PackageInfo {
            name: namer.project_package.clone(),
            version: namer.project_version.clone(),
        }
    }

    /// Create a PackageInfo for the PHP standard library (built-ins).
    pub fn php_stdlib() -> Self {
        PackageInfo {
            name: "php-stdlib".to_string(),
            version: String::new(),
        }
    }
}

/// Main symbol namer that generates SCIP symbol strings.
pub struct SymbolNamer {
    pub project_package: String,
    pub project_version: String,
}

impl SymbolNamer {
    /// Create a new SymbolNamer for a project.
    pub fn new(
        project_package: impl Into<String>,
        project_version: impl Into<String>,
    ) -> Self {
        SymbolNamer {
            project_package: project_package.into(),
            project_version: project_version.into(),
        }
    }

    // ---------------------------------------------------------------
    // Package prefix helpers
    // ---------------------------------------------------------------

    /// Format a SCIP package prefix: `"scip-php composer {name} {version} "`.
    pub fn package_prefix(pkg_name: &str, pkg_version: &str) -> String {
        format!("scip-php composer {} {} ", pkg_name, pkg_version)
    }

    /// Package prefix for the current project.
    pub fn project_prefix(&self) -> String {
        Self::package_prefix(&self.project_package, &self.project_version)
    }

    // ---------------------------------------------------------------
    // Descriptor helpers (static / associated functions)
    // ---------------------------------------------------------------

    /// Convert a PHP FQN to a descriptor path by replacing `\` with `/`.
    pub fn fqn_to_descriptor(fqn: &str) -> String {
        fqn.replace('\\', "/")
    }

    /// Escape a descriptor part with backticks if it contains spaces or backticks.
    pub fn escape_descriptor_part(s: &str) -> String {
        if s.contains(' ') || s.contains('`') {
            format!("`{}`", s.replace('`', "``"))
        } else {
            s.to_string()
        }
    }

    /// Class descriptor: `"Namespace/Class#"`.
    pub fn class_descriptor(fqn: &str) -> String {
        let normalized = fqn.trim_start_matches('\\');
        format!("{}#", Self::fqn_to_descriptor(normalized))
    }

    /// Method descriptor: `"Namespace/Class#methodName()."`.
    pub fn method_descriptor(class_fqn: &str, method_name: &str) -> String {
        let normalized = class_fqn.trim_start_matches('\\');
        format!(
            "{}{}().",
            Self::class_descriptor(normalized),
            Self::escape_descriptor_part(method_name)
        )
    }

    /// Property descriptor: `"Namespace/Class#$propName."`.
    pub fn property_descriptor(class_fqn: &str, prop_name: &str) -> String {
        let normalized = class_fqn.trim_start_matches('\\');
        format!(
            "{}${}.",
            Self::class_descriptor(normalized),
            Self::escape_descriptor_part(prop_name)
        )
    }

    /// Class constant descriptor: `"Namespace/Class#CONST_NAME."`.
    pub fn class_const_descriptor(class_fqn: &str, const_name: &str) -> String {
        let normalized = class_fqn.trim_start_matches('\\');
        format!(
            "{}{}.",
            Self::class_descriptor(normalized),
            Self::escape_descriptor_part(const_name)
        )
    }

    /// Function descriptor: `"Namespace/functionName()."`.
    pub fn function_descriptor(fqn: &str) -> String {
        let normalized = fqn.trim_start_matches('\\');
        format!("{}().", Self::fqn_to_descriptor(normalized))
    }

    /// Local symbol: `"local N"`.
    pub fn local_symbol(counter: u32) -> String {
        format!("local {}", counter)
    }

    // ---------------------------------------------------------------
    // Full symbol construction methods (with package prefix)
    // ---------------------------------------------------------------

    /// Symbol for a project class (uses project package info).
    pub fn symbol_for_class(&self, fqn: &str) -> String {
        let normalized = fqn.trim_start_matches('\\');
        format!(
            "{}{}",
            self.project_prefix(),
            Self::class_descriptor(normalized)
        )
    }

    /// Symbol for a class-like with explicit package info.
    /// Strips leading `\\` before descriptor generation.
    pub fn symbol_for_class_like(
        &self,
        fqn: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        let normalized = fqn.trim_start_matches('\\');
        format!(
            "{}{}",
            Self::package_prefix(pkg_name, pkg_version),
            Self::class_descriptor(normalized)
        )
    }

    /// Symbol for a project class (alias for symbol_for_class).
    pub fn project_class_symbol(&self, fqn: &str) -> String {
        self.symbol_for_class(fqn)
    }

    /// Symbol for a vendor class with explicit package info.
    pub fn symbol_for_vendor_class(
        &self,
        fqn: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        self.symbol_for_class_like(fqn, pkg_name, pkg_version)
    }

    /// Symbol for a built-in PHP class (uses `php-stdlib` package).
    pub fn symbol_for_builtin_class(&self, name: &str) -> String {
        let normalized = name.trim_start_matches('\\');
        format!(
            "{}{}",
            Self::package_prefix("php-stdlib", ""),
            Self::class_descriptor(normalized)
        )
    }

    /// Symbol for a method with explicit package info.
    pub fn symbol_for_method(
        &self,
        class_fqn: &str,
        method_name: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        let normalized = class_fqn.trim_start_matches('\\');
        format!(
            "{}{}",
            Self::package_prefix(pkg_name, pkg_version),
            Self::method_descriptor(normalized, method_name)
        )
    }

    /// Symbol for a constructor (calls symbol_for_method with `__construct`).
    pub fn symbol_for_constructor(
        &self,
        class_fqn: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        self.symbol_for_method(class_fqn, "__construct", pkg_name, pkg_version)
    }

    /// Symbol for a property with explicit package info.
    /// `prop_name` should NOT include the `$` prefix; it is added automatically.
    pub fn symbol_for_property(
        &self,
        class_fqn: &str,
        prop_name: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        let normalized = class_fqn.trim_start_matches('\\');
        format!(
            "{}{}",
            Self::package_prefix(pkg_name, pkg_version),
            Self::property_descriptor(normalized, prop_name)
        )
    }

    /// Symbol for a static property (same format as instance property).
    pub fn symbol_for_static_property(
        &self,
        class_fqn: &str,
        prop_name: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        self.symbol_for_property(class_fqn, prop_name, pkg_name, pkg_version)
    }

    /// Symbol for a class constant with explicit package info.
    pub fn symbol_for_class_const(
        &self,
        class_fqn: &str,
        const_name: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        let normalized = class_fqn.trim_start_matches('\\');
        format!(
            "{}{}",
            Self::package_prefix(pkg_name, pkg_version),
            Self::class_const_descriptor(normalized, const_name)
        )
    }

    /// Symbol for an enum case (same format as class constant).
    pub fn symbol_for_enum_case(
        &self,
        enum_fqn: &str,
        case_name: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        self.symbol_for_class_const(enum_fqn, case_name, pkg_name, pkg_version)
    }

    /// Symbol for a standalone function with explicit package info.
    pub fn symbol_for_function(
        &self,
        fqn: &str,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        let normalized = fqn.trim_start_matches('\\');
        format!(
            "{}{}",
            Self::package_prefix(pkg_name, pkg_version),
            Self::function_descriptor(normalized)
        )
    }

    /// Symbol for a project-level function (uses project package info).
    pub fn project_function_symbol(&self, fqn: &str) -> String {
        self.symbol_for_function(fqn, &self.project_package, &self.project_version)
    }

    /// Symbol for a function parameter. Returns `"local N"`.
    /// The parameter name is not included in the symbol string.
    pub fn symbol_for_param(_param_name: &str, local_id: u32) -> String {
        Self::local_symbol(local_id)
    }

    /// Symbol for a local variable. Returns `"local N"`.
    pub fn symbol_for_local_var(local_id: u32) -> String {
        Self::local_symbol(local_id)
    }

    /// Symbol for an anonymous class, using file path and line number.
    pub fn symbol_for_anonymous_class(
        &self,
        relative_file_path: &str,
        line: u32,
        pkg_name: &str,
        pkg_version: &str,
    ) -> String {
        let descriptor = format!("{}$anonymous_class_L{}#", relative_file_path, line);
        format!(
            "{}{}",
            Self::package_prefix(pkg_name, pkg_version),
            descriptor
        )
    }

    // ---------------------------------------------------------------
    // Package resolution
    // ---------------------------------------------------------------

    /// Resolve which package a file belongs to.
    pub fn resolve_package_for_file(
        &self,
        file_path: &Path,
        composer: &Composer,
    ) -> PackageInfo {
        let (name, version) = composer.package_for_file(file_path);
        PackageInfo::new(name, version)
    }

    /// Resolve the package for a given FQN, checking stubs first, then file-based,
    /// then FQN-based resolution.
    pub fn resolve_package(
        &self,
        fqn: &str,
        file_path: Option<&Path>,
        composer: &Composer,
    ) -> PackageInfo {
        let normalized = fqn.trim_start_matches('\\');

        // 1. Check if it's a built-in class or function
        if composer.stubs.is_builtin_class(normalized)
            || composer.stubs.is_builtin_function(normalized)
        {
            return PackageInfo::php_stdlib();
        }

        // 2. File-based resolution
        if let Some(path) = file_path {
            let (name, version) = composer.package_for_file(path);
            return PackageInfo::new(name, version);
        }

        // 3. FQN-based resolution via package_for_class
        let (name, version) = composer.package_for_class(fqn);
        PackageInfo::new(name, version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn namer() -> SymbolNamer {
        SymbolNamer::new("myvendor/myapp", "1.0.0")
    }

    // ---------------------------------------------------------------
    // Format tests
    // ---------------------------------------------------------------

    #[test]
    fn test_class_symbol() {
        assert_eq!(
            namer().symbol_for_class("App\\Models\\User"),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/User#"
        );
    }

    #[test]
    fn test_global_namespace_class() {
        assert_eq!(
            namer().symbol_for_class("SimpleClass"),
            "scip-php composer myvendor/myapp 1.0.0 SimpleClass#"
        );
    }

    #[test]
    fn test_class_descriptor() {
        assert_eq!(
            SymbolNamer::class_descriptor("App\\Models\\User"),
            "App/Models/User#"
        );
    }

    #[test]
    fn test_method_descriptor() {
        assert_eq!(
            SymbolNamer::method_descriptor("App\\Models\\User", "getName"),
            "App/Models/User#getName()."
        );
    }

    #[test]
    fn test_property_descriptor() {
        assert_eq!(
            SymbolNamer::property_descriptor("App\\Models\\User", "id"),
            "App/Models/User#$id."
        );
    }

    #[test]
    fn test_const_descriptor() {
        assert_eq!(
            SymbolNamer::class_const_descriptor("App\\Models\\Status", "ACTIVE"),
            "App/Models/Status#ACTIVE."
        );
    }

    #[test]
    fn test_function_descriptor() {
        assert_eq!(
            SymbolNamer::function_descriptor("App\\Helpers\\format_date"),
            "App/Helpers/format_date()."
        );
    }

    #[test]
    fn test_local_symbol() {
        assert_eq!(SymbolNamer::local_symbol(0), "local 0");
        assert_eq!(SymbolNamer::local_symbol(42), "local 42");
    }

    // ---------------------------------------------------------------
    // Class symbol tests
    // ---------------------------------------------------------------

    #[test]
    fn test_namespaced_class() {
        assert_eq!(
            namer().symbol_for_class_like("App\\Models\\User", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/User#"
        );
    }

    #[test]
    fn test_deeply_nested_class() {
        assert_eq!(
            namer().symbol_for_class_like("A\\B\\C\\D\\E", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 A/B/C/D/E#"
        );
    }

    #[test]
    fn test_leading_backslash_stripped() {
        assert_eq!(
            namer().symbol_for_class_like("\\App\\Models\\User", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/User#"
        );
    }

    #[test]
    fn test_interface_same_format() {
        // Interfaces use the same symbol format as classes
        assert_eq!(
            namer().symbol_for_class_like("App\\Contracts\\UserRepository", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 App/Contracts/UserRepository#"
        );
    }

    #[test]
    fn test_vendor_class() {
        assert_eq!(
            namer().symbol_for_vendor_class(
                "Illuminate\\Database\\Eloquent\\Model",
                "laravel/framework",
                "v10.0.0"
            ),
            "scip-php composer laravel/framework v10.0.0 Illuminate/Database/Eloquent/Model#"
        );
    }

    // ---------------------------------------------------------------
    // Member symbol tests
    // ---------------------------------------------------------------

    #[test]
    fn test_method_symbol() {
        assert_eq!(
            namer().symbol_for_method(
                "App\\Models\\User",
                "getName",
                "myvendor/myapp",
                "1.0.0"
            ),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/User#getName()."
        );
    }

    #[test]
    fn test_constructor_symbol() {
        assert_eq!(
            namer().symbol_for_constructor("App\\Models\\User", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/User#__construct()."
        );
    }

    #[test]
    fn test_property_symbol() {
        assert_eq!(
            namer().symbol_for_property("App\\Models\\User", "id", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/User#$id."
        );
    }

    #[test]
    fn test_class_const_symbol() {
        assert_eq!(
            namer().symbol_for_class_const(
                "App\\Models\\Status",
                "ACTIVE",
                "myvendor/myapp",
                "1.0.0"
            ),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/Status#ACTIVE."
        );
    }

    #[test]
    fn test_function_symbol() {
        assert_eq!(
            namer().symbol_for_function("App\\Helpers\\format_date", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 App/Helpers/format_date()."
        );
    }

    #[test]
    fn test_global_function() {
        assert_eq!(
            namer().symbol_for_function("strlen", "php-stdlib", ""),
            "scip-php composer php-stdlib  strlen()."
        );
    }

    #[test]
    fn test_local_var_symbol() {
        assert_eq!(SymbolNamer::symbol_for_local_var(0), "local 0");
        assert_eq!(SymbolNamer::symbol_for_local_var(99), "local 99");
    }

    #[test]
    fn test_param_symbol() {
        // The param name is not used in the symbol; only the local_id matters
        assert_eq!(SymbolNamer::symbol_for_param("name", 3), "local 3");
    }

    // ---------------------------------------------------------------
    // Package resolution tests
    // ---------------------------------------------------------------

    #[test]
    fn test_package_info_new() {
        let pkg = PackageInfo::new("vendor/pkg", "2.0.0");
        assert_eq!(pkg.name, "vendor/pkg");
        assert_eq!(pkg.version, "2.0.0");
    }

    #[test]
    fn test_package_info_project() {
        let n = namer();
        let pkg = PackageInfo::project(&n);
        assert_eq!(pkg.name, "myvendor/myapp");
        assert_eq!(pkg.version, "1.0.0");
    }

    #[test]
    fn test_php_stdlib() {
        let pkg = PackageInfo::php_stdlib();
        assert_eq!(pkg.name, "php-stdlib");
        assert_eq!(pkg.version, "");
    }

    // ---------------------------------------------------------------
    // Builtin class symbol test
    // ---------------------------------------------------------------

    #[test]
    fn test_builtin_class_symbol() {
        assert_eq!(
            namer().symbol_for_builtin_class("DateTime"),
            "scip-php composer php-stdlib  DateTime#"
        );
    }

    // ---------------------------------------------------------------
    // Anonymous class test
    // ---------------------------------------------------------------

    #[test]
    fn test_anonymous_class_symbol() {
        assert_eq!(
            namer().symbol_for_anonymous_class("src/Foo.php", 42, "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 src/Foo.php$anonymous_class_L42#"
        );
    }

    // ---------------------------------------------------------------
    // Escape descriptor part tests
    // ---------------------------------------------------------------

    #[test]
    fn test_escape_simple_name() {
        assert_eq!(SymbolNamer::escape_descriptor_part("getName"), "getName");
    }

    #[test]
    fn test_escape_name_with_space() {
        assert_eq!(
            SymbolNamer::escape_descriptor_part("get name"),
            "`get name`"
        );
    }

    #[test]
    fn test_escape_name_with_backtick() {
        assert_eq!(
            SymbolNamer::escape_descriptor_part("get`name"),
            "`get``name`"
        );
    }

    // ---------------------------------------------------------------
    // Project-level convenience methods
    // ---------------------------------------------------------------

    #[test]
    fn test_project_class_symbol() {
        assert_eq!(
            namer().project_class_symbol("App\\Services\\Auth"),
            "scip-php composer myvendor/myapp 1.0.0 App/Services/Auth#"
        );
    }

    #[test]
    fn test_project_function_symbol() {
        assert_eq!(
            namer().project_function_symbol("App\\Helpers\\dd"),
            "scip-php composer myvendor/myapp 1.0.0 App/Helpers/dd()."
        );
    }

    // ---------------------------------------------------------------
    // Enum case test
    // ---------------------------------------------------------------

    #[test]
    fn test_enum_case_symbol() {
        assert_eq!(
            namer().symbol_for_enum_case("App\\Enums\\Color", "Red", "myvendor/myapp", "1.0.0"),
            "scip-php composer myvendor/myapp 1.0.0 App/Enums/Color#Red."
        );
    }

    // ---------------------------------------------------------------
    // Static property test
    // ---------------------------------------------------------------

    #[test]
    fn test_static_property_symbol() {
        assert_eq!(
            namer().symbol_for_static_property(
                "App\\Models\\User",
                "instance",
                "myvendor/myapp",
                "1.0.0"
            ),
            "scip-php composer myvendor/myapp 1.0.0 App/Models/User#$instance."
        );
    }
}
