---
layout: page
title: Playground
---

<script setup>
import { defineClientComponent } from 'vitepress'

const DewPlayground = defineClientComponent(() =>
  import('./.vitepress/playground/components/DewPlayground.vue')
)
</script>

# Playground

Try Dew expressions in the browser. Select a domain profile, type an expression, and see the generated code for each backend.

<DewPlayground />
