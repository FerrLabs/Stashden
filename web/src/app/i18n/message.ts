export type Plural = {
  readonly one: string;
  readonly other: string;
};

export type Message = string | Plural;

export type Value = string | number;

type Placeholders<Text extends string> = Text extends `${string}{${infer Name}}${infer Rest}`
  ? Name | Placeholders<Rest>
  : never;

export type Names<M extends Message> = M extends string
  ? Placeholders<M>
  : M extends Plural
    ? Placeholders<M['one'] | M['other']> | 'count'
    : never;

export type Values<M extends Message> = { readonly [Name in Names<M>]: Value };

export type Mirror<Source> = { readonly [Key in keyof Source]: Message };
