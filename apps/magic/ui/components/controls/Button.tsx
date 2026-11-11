import React from "react";
import { Icon, IconName } from "./Icon";

export type ControlVariant = "default" | "primary" | "ghost" | "subtle" | "danger";
export type ControlSize = "xs" | "sm" | "md";

export interface ButtonProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  variant?: ControlVariant;
  size?: ControlSize;
  icon?: IconName | React.ReactNode;
  trailingIcon?: IconName | React.ReactNode;
  /** Single-word interactive label (e.g. "Save", "Preview", "Rig", "Open"). */
  label?: string;
  /** Optional compact count or badge chip rendered beside the single-word label. */
  badge?: number | string;
  active?: boolean;
  fullWidth?: boolean;
}

function renderIcon(slot?: IconName | React.ReactNode) {
  if (!slot) return null;
  if (typeof slot === "string") {
    return <Icon name={slot as IconName} />;
  }
  return <span className="ui-btn-custom-icon" aria-hidden="true">{slot}</span>;
}

/**
 * Canonical Magic Studio interactive button.
 * Enforces predefined responsive height, horizontal/vertical padding, zero vertical margin,
 * and icon + single-word label alignment across every toolbar, rail, and inspector.
 */
export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = "default",
      size = "md",
      icon,
      trailingIcon,
      label,
      badge,
      active = false,
      fullWidth = false,
      className = "",
      type = "button",
      ...rest
    },
    ref,
  ) => {
    const iconOnly = Boolean((icon || trailingIcon) && !label && badge === undefined);
    const classes = [
      "ui-btn",
      `ui-btn--${variant}`,
      `ui-btn--${size}`,
      iconOnly ? "ui-btn--icon" : "",
      active ? "is-active" : "",
      fullWidth ? "ui-btn--full" : "",
      className,
    ]
      .filter(Boolean)
      .join(" ");

    return (
      <button ref={ref} type={type} className={classes} {...rest}>
        {renderIcon(icon)}
        {label && <span className="ui-btn-label">{label}</span>}
        {badge !== undefined && badge !== null && badge !== "" && (
          <span className="ui-btn-badge">{badge}</span>
        )}
        {renderIcon(trailingIcon)}
      </button>
    );
  },
);
Button.displayName = "Button";

export interface IconButtonProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  icon: IconName | React.ReactNode;
  label: string;
  variant?: ControlVariant;
  size?: ControlSize;
  active?: boolean;
}

export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ icon, label, variant = "ghost", size = "md", active = false, title, ...rest }, ref) => (
    <Button
      ref={ref}
      icon={icon}
      variant={variant}
      size={size}
      active={active}
      aria-label={rest["aria-label"] ?? label}
      title={title ?? label}
      {...rest}
    />
  ),
);
IconButton.displayName = "IconButton";
