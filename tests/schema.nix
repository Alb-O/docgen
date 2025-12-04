# Tests for schema definitions
{
  schema,
  ...
}:
{
  # Test that defaults exist
  defaults."test files defaults exist" = {
    expr = schema.defaults.files;
    expected = {
      title = "File Reference";
      titleLevel = 1;
    };
  };

  defaults."test methods defaults exist" = {
    expr = schema.defaults.methods;
    expected = {
      title = "API Methods";
      titleLevel = 1;
    };
  };

  defaults."test options defaults exist" = {
    expr = schema.defaults.options;
    expected = {
      title = "Module Options";
      anchorPrefix = "opt-";
    };
  };

  # Test that example manifest is valid
  example."test example has files config" = {
    expr = schema.example.files.title;
    expected = "File Reference";
  };

  example."test example has methods config" = {
    expr = schema.example.methods.title;
    expected = "API Methods";
  };

  example."test example has options config" = {
    expr = schema.example.options.title;
    expected = "Module Options";
  };

  example."test example files has sections" = {
    expr = builtins.length schema.example.files.sections;
    expected = 1;
  };

  example."test example methods has sections" = {
    expr = builtins.length schema.example.methods.sections;
    expected = 2;
  };

  # Test that types are exported
  types."test fileEntryType exists" = {
    expr = schema.types ? fileEntryType;
    expected = true;
  };

  types."test filesSectionType exists" = {
    expr = schema.types ? filesSectionType;
    expected = true;
  };

  types."test methodsSectionType exists" = {
    expr = schema.types ? methodsSectionType;
    expected = true;
  };

  types."test filesConfigType exists" = {
    expr = schema.types ? filesConfigType;
    expected = true;
  };

  types."test methodsConfigType exists" = {
    expr = schema.types ? methodsConfigType;
    expected = true;
  };

  types."test optionsConfigType exists" = {
    expr = schema.types ? optionsConfigType;
    expected = true;
  };

  types."test manifestType exists" = {
    expr = schema.types ? manifestType;
    expected = true;
  };
}
