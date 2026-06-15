import { ArrowCircleUp, type Icon } from "@phosphor-icons/react";
import {
  LayoutGroup,
  motion,
  useReducedMotion,
  type Transition,
} from "framer-motion";
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

const NAVIGATION_INDICATOR_TRANSITION: Transition = {
  duration: 0.24,
  ease: [0.22, 1, 0.36, 1],
};

function getNavigationItemClass(isActive: boolean) {
  return cn("navigation-item", isActive && "navigation-item--active");
}

export function NavigationGroup({
  children,
  id,
}: {
  children: ReactNode;
  id: string;
}) {
  return <LayoutGroup id={id}>{children}</LayoutGroup>;
}

interface NavigationItemContentProps {
  active?: boolean;
  activeIndicatorLayoutId?: string;
  badge?: ReactNode;
  icon: Icon;
  label: ReactNode;
  trailing?: ReactNode;
}

interface NavigationItemVisualProps extends NavigationItemContentProps {
  className?: string;
}

interface NavigationItemButtonProps
  extends NavigationItemVisualProps,
    Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "className"> {}

interface NavigationItemAnchorProps
  extends NavigationItemVisualProps,
    Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "children" | "className"> {}

interface NavigationItemLinkProps
  extends Omit<NavigationItemVisualProps, "active">,
    Omit<NavLinkProps, "children" | "className"> {
  activeWhen?: (isActive: boolean) => boolean;
}

export function NavigationItemButton({
  active = false,
  activeIndicatorLayoutId,
  badge,
  className,
  icon,
  label,
  trailing,
  type = "button",
  ...props
}: NavigationItemButtonProps) {
  return (
    <button
      className={cn(getNavigationItemClass(active), className)}
      type={type}
      {...props}
    >
      <NavigationItemContent
        active={active}
        activeIndicatorLayoutId={activeIndicatorLayoutId}
        badge={badge}
        icon={icon}
        label={label}
        trailing={trailing}
      />
    </button>
  );
}

export function NavigationItemAnchor({
  active = false,
  activeIndicatorLayoutId,
  badge,
  className,
  icon,
  label,
  trailing,
  ...props
}: NavigationItemAnchorProps) {
  return (
    <a className={cn(getNavigationItemClass(active), className)} {...props}>
      <NavigationItemContent
        active={active}
        activeIndicatorLayoutId={activeIndicatorLayoutId}
        badge={badge}
        icon={icon}
        label={label}
        trailing={trailing}
      />
    </a>
  );
}

export function NavigationItemLink({
  activeIndicatorLayoutId,
  activeWhen = (isActive) => isActive,
  badge,
  className,
  icon,
  label,
  trailing,
  ...props
}: NavigationItemLinkProps) {
  return (
    <NavLink
      className={({ isActive }) =>
        cn(getNavigationItemClass(activeWhen(isActive)), className)
      }
      {...props}
    >
      {({ isActive }) => {
        const active = activeWhen(isActive);

        return (
          <NavigationItemContent
            active={active}
            activeIndicatorLayoutId={activeIndicatorLayoutId}
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

function NavigationItemContent({
  active = false,
  activeIndicatorLayoutId,
  badge,
  icon: IconComponent,
  label,
  trailing,
}: NavigationItemContentProps) {
  return (
    <>
      {active && activeIndicatorLayoutId ? (
        <NavigationItemActiveIndicator layoutId={activeIndicatorLayoutId} />
      ) : null}
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
    </>
  );
}

function NavigationItemActiveIndicator({ layoutId }: { layoutId: string }) {
  const prefersReducedMotion = useReducedMotion();

  return (
    <motion.span
      aria-hidden="true"
      className="navigation-item__active-indicator"
      layoutId={layoutId}
      transition={
        prefersReducedMotion
          ? { duration: 0 }
          : NAVIGATION_INDICATOR_TRANSITION
      }
    />
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
