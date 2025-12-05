{
  /**
    Doc-comment
  */
  omited =
    # Not visible
    arg: 1;

  /**
    Doc-comment
  */
  multiple =
    # Not visible
    arg:
    # Not visible
    foo:
    /**
      Not visible
    */
    bar:
    1;

  /**
    Doc-comment before the lambda causes the whole
    lambda including its arguments to use doc-comment style
  */
  argumentTest =
    {
      /**
        First formal
      */
      formal1,
      /**
        Second formal
      */
      formal2,
      /**
        Third formal
      */
      formal3,
      /**
        Fourth formal
      */
      formal4,
    }:
    { };
}
