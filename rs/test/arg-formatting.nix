{
  /**
    Documented function with various argument formats.
  */
  fn =
    /**
      Single argument
    */
    a:
    /**
      Structured function argument

      `default`
      : documented argument

      `example`
      : i like this argument. another!
    */
    {
      default ? null,
      example ? null,
    }@args:
    0;
}
