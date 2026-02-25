import type { ButtonHTMLAttributes } from "react";

export type ButtonVariant = "default" | "primary" | "success" | "warning" | "danger";
export type ButtonSize = "default" | "large" | "small";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  plain?: boolean;
}

const variantClass: Record<ButtonVariant, string> = {
  default: "btn-default",
  primary: "btn-primary",
  success: "btn-success",
  warning: "btn-warning",
  danger: "btn-danger",
};

const sizeClass: Record<ButtonSize, string> = {
  default: "",
  large: "btn-large",
  small: "btn-small",
};

export function Button({
  variant = "default",
  size = "default",
  plain = false,
  className = "",
  disabled,
  ...rest
}: ButtonProps) {
  const classes = [
    "btn",
    variantClass[variant],
    sizeClass[size],
    plain ? "btn-plain" : "",
    disabled ? "is-disabled" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return <button type="button" className={classes} disabled={disabled} {...rest} />;
}
