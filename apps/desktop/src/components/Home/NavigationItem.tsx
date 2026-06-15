import { ArrowCircleUp, type Icon } from "@phosphor-icons/react";
import {
  NavLink,
  type NavLinkProps,
} from "react-router-dom";
import type {
  AnchorHTMLAttributes,
  ButtonHTMLAttributes,
  ReactNode,
} from "react";

import { cn } from "@/lib/utils";

function getNavigationItemClass(isActive: boolean) {
  return cn("navigation-item", isActive && "navigation-item--active");
}

interface NavigationProps {
  className?: string;
  id: string;
  items: NavigationItemConfig[];
}

interface NavigationItemBaseConfig {
  badge?: ReactNode;
  className?: string;
  icon: Icon;
  id: string;
  label: ReactNode;
  testId?: string;
  trailing?: ReactNode;
}

interface NavigationItemButtonConfig
  extends NavigationItemBaseConfig,
    Omit<
      ButtonHTMLAttributes<HTMLButtonElement>,
      "children" | "className" | "id"
    > {
  active?: boolean;
  kind: "button";
}

interface NavigationItemAnchorConfig
  extends NavigationItemBaseConfig,
    Omit<
      AnchorHTMLAttributes<HTMLAnchorElement>,
      "children" | "className" | "id"
    > {
  active?: boolean;
  kind: "anchor";
}

interface NavigationItemLinkConfig
  extends NavigationItemBaseConfig,
    Omit<NavLinkProps, "children" | "className" | "id"> {
  activeWhen?: (isActive: boolean) => boolean;
  kind: "link";
}

export type NavigationItemConfig =
  | NavigationItemAnchorConfig
  | NavigationItemButtonConfig
  | NavigationItemLinkConfig;

export function Navigation({ className, id, items }: NavigationProps) {
  return (
    <div className={className} id={id}>
      {items.map((item) => (
        <NavigationItem item={item} key={item.id} />
      ))}
    </div>
  );
}

interface NavigationItemProps {
  item: NavigationItemConfig;
}

function NavigationItem({ item }: NavigationItemProps) {
  switch (item.kind) {
    case "anchor":
      return <NavigationItemAnchor item={item} />;
    case "button":
      return <NavigationItemButton item={item} />;
    case "link":
      return <NavigationItemLink item={item} />;
  }
}

interface NavigationItemRendererProps<TItem extends NavigationItemConfig> {
  item: TItem;
}

function NavigationItemButton({
  item,
}: NavigationItemRendererProps<NavigationItemButtonConfig>) {
  const {
    active = false,
    badge,
    className,
    icon,
    kind: _kind,
    label,
    testId,
    trailing,
    type = "button",
    ...props
  } = item;

  return (
    <button
      className={cn(getNavigationItemClass(active), className)}
      data-testid={testId}
      type={type}
      {...props}
    >
      <NavigationItemContent
        active={active}
        badge={badge}
        icon={icon}
        label={label}
        trailing={trailing}
      />
    </button>
  );
}

function NavigationItemAnchor({
  item,
}: NavigationItemRendererProps<NavigationItemAnchorConfig>) {
  const {
    active = false,
    badge,
    className,
    icon,
    kind: _kind,
    label,
    testId,
    trailing,
    ...props
  } = item;

  return (
    <a
      className={cn(getNavigationItemClass(active), className)}
      data-testid={testId}
      {...props}
    >
      <NavigationItemContent
        active={active}
        badge={badge}
        icon={icon}
        label={label}
        trailing={trailing}
      />
    </a>
  );
}

function NavigationItemLink({
  item,
}: NavigationItemRendererProps<NavigationItemLinkConfig>) {
  const {
    activeWhen = (isActive) => isActive,
    badge,
    className,
    icon,
    kind: _kind,
    label,
    testId,
    trailing,
    ...props
  } = item;

  return (
    <NavLink
      className={({ isActive }) =>
        cn(getNavigationItemClass(activeWhen(isActive)), className)
      }
      data-testid={testId}
      {...props}
    >
      {({ isActive }) => {
        const active = activeWhen(isActive);

        return (
          <NavigationItemContent
            active={active}
            badge={badge}
            icon={icon}
            label={label}
            trailing={trailing}
          />
        );
      }}
    </NavLink>
  );
}

interface NavigationItemContentProps {
  active?: boolean;
  badge?: ReactNode;
  icon: Icon;
  label: ReactNode;
  trailing?: ReactNode;
}

function NavigationItemContent({
  active = false,
  badge,
  icon: IconComponent,
  label,
  trailing,
}: NavigationItemContentProps) {
  return (
    <span className="navigation-item__content">
      <IconComponent
        className="navigation-item__icon"
        weight={active ? "duotone" : "regular"}
      />
      <span className="navigation-item__label-frame">
        <span className="navigation-item__label">{label}</span>
      </span>
      {badge}
      {trailing}
    </span>
  );
}

export function NavigationAttentionBadge() {
  return (
    <ArrowCircleUp
      className="h-4 w-4 shrink-0 text-green-500"
      weight="fill"
    />
  );
}
