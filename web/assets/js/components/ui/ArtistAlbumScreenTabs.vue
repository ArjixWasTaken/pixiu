<template>
  <div class="tabs">
    <header>
      <slot name="header" />
    </header>
    <main>
      <slot />
    </main>
  </div>
</template>

<style lang="postcss" scoped>
.tabs {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

header {
  flex-shrink: 0;
  padding: 0 24px;
  border-bottom: 1px solid var(--schemes-outline-variant);

  @media (max-width: 768px) {
    padding: 0 4px;
  }
}

:deep(header ul) {
  display: flex;
  max-width: 560px;
  overflow-x: auto;
  scrollbar-width: none;

  li {
    flex: 1 0 auto;
    list-style: none;

    a {
      position: relative;
      display: flex;
      justify-content: center;
      align-items: center;
      height: 48px;
      padding: 0 16px;
      color: var(--schemes-on-surface-variant);
      font-size: calc(var(--static-title-small-size) * 1px);
      line-height: calc(var(--static-title-small-line-height) * 1px);
      letter-spacing: calc(var(--static-title-small-tracking) * 1px);
      font-weight: 500;
      white-space: nowrap;

      &:hover {
        color: var(--schemes-on-surface);
        background: color-mix(in srgb, var(--schemes-on-surface) 8%, transparent);
      }
    }

    &.active a {
      color: var(--schemes-primary);

      &::after {
        content: '';
        position: absolute;
        left: 16px;
        right: 16px;
        bottom: 0;
        height: 3px;
        border-radius: 3px 3px 0 0;
        background: var(--schemes-primary);
      }
    }
  }
}

main {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: auto;
}

:deep(:is(.songs-pane, .albums-pane)) {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: auto;
}

:deep(.albums-pane .none) {
  padding: 16px 24px;
}

:deep(.info-pane) {
  max-width: 72ch;
  padding: 20px 24px;
}
</style>
