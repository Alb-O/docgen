{
  # not a doc comment
  hidden = a: a;

  /**
    doc comment in markdown format

    # Example

    This is a parsed example
  */
  docComment = { };

  /**
    another doc comment
  */
  rfc-style = { };

  # Omitting a doc comment from an attribute doesn't duplicate the previous one
  /**
    Comment
  */
  foo = 0;

  # This should not have any docs
  bar = 1;

}
