export type RequiredField = {
  key: string;
  label: string;
  value: string | null | undefined;
  required?: boolean;
};

export type FieldErrors = Record<string, string>;

export function requiredFieldErrors(fields: RequiredField[]): FieldErrors {
  return Object.fromEntries(fields
    .filter((field) => field.required !== false && !field.value?.trim())
    .map((field) => [field.key, `${field.label} is required.`]));
}

export function firstFieldError(errors: FieldErrors): string | null {
  return Object.keys(errors)[0] ?? null;
}
