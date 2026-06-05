/**
 * Type declarations for @omnilang/test-utils-vue
 */
import type { VueWrapper, MountingOptions } from "@vue/test-utils";
import type { Component } from "vue";

export interface OmniMountOptions extends MountingOptions<unknown> {
  /** Component props */
  props?: Record<string, unknown>;
  /** Pinia store mocks: { storeName: { field: value, ... } } */
  store?: Record<string, Record<string, unknown>>;
  /** Additional global configuration */
  globals?: Record<string, unknown>;
}

export interface OmniVueWrapper extends VueWrapper<unknown> {
  /** Find element by data-testid attribute */
  findByTestId(testId: string): ReturnType<VueWrapper["find"]>;
  /** Find all elements matching a data-testid */
  findAllByTestId(testId: string): ReturnType<VueWrapper["findAll"]>;
  /** Get text content by test-id, or null if not found */
  getByTestId(testId: string): string | null;
  /** Check if element with test-id exists */
  hasTestId(testId: string): boolean;
}

export function mountOmni(
  component: Component,
  options?: OmniMountOptions
): OmniVueWrapper;

export function fillInput(
  wrapper: OmniVueWrapper,
  testId: string,
  value: string
): Promise<void>;

export function toggleCheckbox(
  wrapper: OmniVueWrapper,
  testId: string,
  checked?: boolean
): Promise<void>;

export function clickButton(
  wrapper: OmniVueWrapper,
  testId: string
): Promise<void>;

export function mockStore(
  storeName: string,
  initialState?: Record<string, unknown>,
  actions?: Record<string, (...args: unknown[]) => unknown>
): Record<string, Record<string, unknown>>;

export function flushPromises(): Promise<void>;
