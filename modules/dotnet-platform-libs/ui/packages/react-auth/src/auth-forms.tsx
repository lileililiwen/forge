// Auth page composition. Every control here comes from @platform/react-ui; this file
// declares no raw input element and no aria-describedby. Field already owns that wiring,
// so a second copy of it would be a second accessibility contract that fails differently
// from the one the rest of the platform already depends on.

import * as React from 'react';
import { Button, Card, Field, State } from '@platform/react-ui';

export type AuthFormState = 'idle' | 'invalid' | 'submitting' | 'success' | 'error';

export interface AuthFormValues {
  email: string;
  password: string;
  confirmPassword?: string;
}

export type AuthFormResult = { ok: true } | { ok: false; message: string };

export interface AuthFormProps {
  /** Return ok:false to move the form to `error`; a throw is caught the same way. */
  onSubmit: (values: AuthFormValues) => Promise<AuthFormResult>;
  submitLabel?: string;
  busyLabel?: string;
  successMessage?: string;
  disabled?: boolean;
}

export function validateAuthForm(values: AuthFormValues): Record<string, string> {
  const errors: Record<string, string> = {};
  if (values.email.trim().length === 0) errors.email = 'Enter your email address.';
  if (values.password.length === 0) errors.password = 'Enter your password.';
  if (values.confirmPassword !== undefined && values.confirmPassword !== values.password) {
    errors.confirmPassword = 'The two passwords do not match.';
  }
  return errors;
}

export function useAuthFormState(onSubmit: AuthFormProps['onSubmit']) {
  const [state, setState] = React.useState<AuthFormState>('idle');
  const [errors, setErrors] = React.useState<Record<string, string>>({});
  const [failure, setFailure] = React.useState<string>('');

  const run = React.useCallback(
    async (values: AuthFormValues) => {
      const invalid = validateAuthForm(values);
      if (Object.keys(invalid).length > 0) {
        setErrors(invalid);
        setState('invalid');
        return;
      }
      setErrors({});
      setFailure('');
      setState('submitting');
      try {
        const result = await onSubmit(values);
        if (result.ok) setState('success');
        else {
          setFailure(result.message);
          setState('error');
        }
      } catch (thrown) {
        setFailure(thrown instanceof Error ? thrown.message : 'The request could not be completed.');
        setState('error');
      }
    },
    [onSubmit],
  );

  return { state, errors, failure, run };
}

interface FormShellProps extends AuthFormProps {
  title: string;
  description: string;
  state: AuthFormState;
  failure: string;
  children: React.ReactNode;
  onSubmitForm: (event: React.FormEvent<HTMLFormElement>) => void;
}

function FormShell({ title, description, state, failure, children, onSubmitForm, submitLabel, busyLabel, successMessage, disabled }: FormShellProps) {
  return (
    <Card className="platform-auth-card">
      <h1 className="platform-auth-title">{title}</h1>
      <p className="platform-auth-description">{description}</p>
      <form className="platform-auth-form" onSubmit={onSubmitForm} noValidate>
        {children}
        {/* State already carries role="alert" for kind="error", so the failure is announced. */}
        {state === 'error' && <State kind="error">{failure}</State>}
        {state === 'success' && <State kind="loading">{successMessage ?? 'Done.'}</State>}
        {/* Button supplies aria-busy and blocks a duplicate submit while loading. */}
        <Button type="submit" loading={state === 'submitting'} disabled={disabled}>
          {state === 'submitting' ? (busyLabel ?? 'Working…') : (submitLabel ?? 'Continue')}
        </Button>
      </form>
    </Card>
  );
}

export function LoginForm({ onSubmit, submitLabel, busyLabel, successMessage, disabled }: AuthFormProps) {
  const { state, errors, failure, run } = useAuthFormState(onSubmit);
  const [values, setValues] = React.useState<AuthFormValues>({ email: '', password: '' });
  const submit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void run(values);
  };
  return (
    <FormShell
      title="Sign in"
      description="Use the account this site created for you."
      state={state}
      failure={failure}
      onSubmitForm={submit}
      submitLabel={submitLabel ?? 'Sign in'}
      busyLabel={busyLabel}
      successMessage={successMessage ?? 'Signed in.'}
      disabled={disabled}
    >
      <Field
        label="Email"
        name="email"
        type="email"
        autoComplete="username"
        value={values.email}
        error={errors.email}
        onChange={(event) => setValues({ ...values, email: event.target.value })}
      />
      <Field
        label="Password"
        name="password"
        type="password"
        autoComplete="current-password"
        value={values.password}
        error={errors.password}
        onChange={(event) => setValues({ ...values, password: event.target.value })}
      />
    </FormShell>
  );
}

export function RegisterForm({ onSubmit, submitLabel, busyLabel, successMessage, disabled }: AuthFormProps) {
  const { state, errors, failure, run } = useAuthFormState(onSubmit);
  const [values, setValues] = React.useState<AuthFormValues>({ email: '', password: '', confirmPassword: '' });
  const submit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void run(values);
  };
  return (
    <FormShell
      title="Create an account"
      description="Registration stays closed until the site owner enables it."
      state={state}
      failure={failure}
      onSubmitForm={submit}
      submitLabel={submitLabel ?? 'Create account'}
      busyLabel={busyLabel}
      successMessage={successMessage ?? 'Check your email to finish signing up.'}
      disabled={disabled}
    >
      <Field
        label="Email"
        name="email"
        type="email"
        autoComplete="email"
        value={values.email}
        error={errors.email}
        onChange={(event) => setValues({ ...values, email: event.target.value })}
      />
      <Field
        label="Password"
        name="password"
        type="password"
        autoComplete="new-password"
        value={values.password}
        error={errors.password}
        onChange={(event) => setValues({ ...values, password: event.target.value })}
      />
      <Field
        label="Confirm password"
        name="confirmPassword"
        type="password"
        autoComplete="new-password"
        value={values.confirmPassword}
        error={errors.confirmPassword}
        onChange={(event) => setValues({ ...values, confirmPassword: event.target.value })}
      />
    </FormShell>
  );
}

export function ForgotPasswordForm({ onSubmit, submitLabel, busyLabel, successMessage, disabled }: AuthFormProps) {
  const { state, errors, failure, run } = useAuthFormState(onSubmit);
  const [values, setValues] = React.useState<AuthFormValues>({ email: '', password: '' });
  const submit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void run(values);
  };
  return (
    <FormShell
      title="Reset your password"
      description="The reply is the same whether or not the address is known."
      state={state}
      failure={failure}
      onSubmitForm={submit}
      submitLabel={submitLabel ?? 'Send reset link'}
      busyLabel={busyLabel}
      successMessage={successMessage ?? 'If that address exists, a reset link is on its way.'}
      disabled={disabled}
    >
      <Field
        label="Email"
        name="email"
        type="email"
        autoComplete="email"
        value={values.email}
        error={errors.email}
        onChange={(event) => setValues({ ...values, email: event.target.value })}
      />
    </FormShell>
  );
}

export interface LogoutControlProps {
  /** An explicit sign-out handler, not a link: the site decides what sign-out means. */
  onLogout: () => void;
  label?: string;
}

export function LogoutControl({ onLogout, label = 'Sign out' }: LogoutControlProps) {
  return (
    <Button type="button" variant="outline" onClick={onLogout}>
      {label}
    </Button>
  );
}
