import type { OpenSlideConfig } from '@open-slide/core';

const openSlideConfig: OpenSlideConfig = {
  ...(process.env.OPEN_SLIDE_BASE ? { base: process.env.OPEN_SLIDE_BASE } : {}),
};

export default openSlideConfig;
