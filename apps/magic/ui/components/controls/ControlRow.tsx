import React from "react";
import { ControlSize } from "./Button";
import { Icon, IconName } from "./Icon";

export interface ToolbarProps extends React.HTMLAttributes<HTMLDivElement> {
  align?: "between" | "start" | "end";
  dense?: boolean;
  wrap?: boolean;
}

export const Toolbar: React.FC<ToolbarProps> = ({
  align = "between",
  dense = false,
  wrap = true,
  className = "",
  children,
  ...rest
}) => (
  <div
    className={[
      "ui-toolbar",
      `ui-toolbar--${align}`,
      dense ? "ui-toolbar--dense" : "",
      wrap ? "ui-toolbar--wrap" : "",
      className,
    ]
      .filter(Boolean)
      .join(" ")}
    {...rest}
  >
    {children}
  </div>
);

export const ControlGroup: React.FC<React.HTMLAttributes<HTMLDivElement>> = ({
  className = "",
  children,
  ...rest
}) => (
  <div className={`ui-control-group ${className}`.trim()} {...rest}>
    {children}
  </div>
);

export interface SegmentedOption<T extends string> {
  value: T;
  /** Single-word segment label */
  label: string;
  icon?: IconName;
  badge?: number | string;
  id?: string;
  title?: string;
  disabled?: boolean;
}

export interface SegmentedProps<T extends string> {
  id?: string;
  value: T;
  options: Array<SegmentedOption<T>>;
  onChange: (value: T) => void;
  size?: ControlSize;
  fullWidth?: boolean;
  ariaLabel?: string;
  className?: string;
}

export function Segmented<T extends string>({
  id,
  value,
  options,
  onChange,
  size = "sm",
  fullWidth = false,
  ariaLabel,
  className = "",
}: SegmentedProps<T>) {
  return (
    <div
      id={id}
      role="tablist"
      aria-label={ariaLabel}
      className={[
        "ui-segmented",
        `ui-segmented--${size}`,
        fullWidth ? "ui-segmented--full" : "",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {options.map((opt) => {
        const selected = opt.value === value;
        return (
          <button
            key={opt.value}
            id={opt.id}
            type="button"
            role="tab"
            aria-selected={selected}
            disabled={opt.disabled}
            title={opt.title ?? opt.label}
            className={`ui-segmented-item ${selected ? "is-active" : ""}`.trim()}
            onClick={() => onChange(opt.value)}
          >
            {opt.icon && <Icon name={opt.icon} />}
            <span className="ui-btn-label">{opt.label}</span>
            {opt.badge !== undefined && <span className="ui-btn-badge">{opt.badge}</span>}
          </button>
        );
      })}
    </div>
  );
}

export interface SelectControlProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  controlSize?: ControlSize;
}

export const SelectControl = React.forwardRef<HTMLSelectElement, SelectControlProps>(
  ({ controlSize = "md", className = "", children, ...rest }, ref) => (
    <select ref={ref} className={`ui-select ui-select--${controlSize} ${className}`.trim()} {...rest}>
      {children}
    </select>
  ),
);
SelectControl.displayName = "SelectControl";

export interface InputControlProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size"> {
  controlSize?: ControlSize;
}

export const InputControl = React.forwardRef<HTMLInputElement, InputControlProps>(
  ({ controlSize = "md", className = "", ...rest }, ref) => (
    <input ref={ref} className={`ui-input ui-input--${controlSize} ${className}`.trim()} {...rest} />
  ),
);
InputControl.displayName = "InputControl";
