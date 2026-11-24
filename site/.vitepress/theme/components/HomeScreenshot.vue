<template>
  <section class="screenshot">
    <div class="frame">
      <img src="/panel-dashboard.webp" width="1600" height="801" alt="Het dashboard van het Stacklane-klantpanel" />
    </div>
  </section>
</template>

<style scoped>
.screenshot {
  margin: 0 auto;
  max-width: 1152px;
  padding: 0 24px 96px;
}

.frame {
  overflow: hidden;
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  background: #18181b;
  padding: 8px;
  box-shadow: 0 30px 60px -20px rgba(24, 24, 27, 0.35);
}

.frame img {
  display: block;
  width: 100%;
  height: auto;
  border-radius: 10px;
}
</style>
