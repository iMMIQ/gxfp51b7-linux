// SPDX-License-Identifier: LGPL-3.0-or-later
#include "chicago/goodix-chicago-runtime.h"
#include <gio/gio.h>
#include <stdint.h>
#include <string.h>

typedef struct {
  GBytes *calibration;
  GoodixChicagoPreprocessor *preprocessor;
  GoodixChicagoEnrollment *gallery;
  GoodixChicagoEngineEnrollmentPolicy policy;
  GoodixChicagoRuntimeProbe *deferred;
} GxfpChicago;

typedef struct {
  int32_t reject, score;
  uint32_t features, quality, coverage, count, accepted, position_x, position_y,
      position_reject;
} GxfpResult;

static void error_text(GError *error, char *out, size_t capacity) {
  if (out && capacity)
    g_strlcpy(out, error ? error->message : "Chicago operation failed",
              capacity);
  g_clear_error(&error);
}

void gxfp_chicago_free(GxfpChicago *ctx) {
  if (!ctx)
    return;
  g_clear_pointer(&ctx->deferred, goodix_chicago_runtime_probe_free);
  g_clear_pointer(&ctx->gallery, goodix_chicago_enrollment_free);
  g_clear_pointer(&ctx->preprocessor, goodix_chicago_preprocessor_free);
  g_clear_pointer(&ctx->calibration, g_bytes_unref);
  g_free(ctx);
}

GxfpChicago *gxfp_chicago_new(const uint16_t *background,
                              const uint8_t *calibration,
                              size_t calibration_len, const uint8_t *gallery,
                              size_t gallery_len, char *error_out,
                              size_t error_capacity) {
  GError *error = NULL;
  GxfpChicago *ctx = g_new0(GxfpChicago, 1);
  ctx->calibration = calibration_len
                         ? g_bytes_new(calibration, calibration_len)
                         : goodix_chicago_calibration_generate(background);
  ctx->preprocessor =
      goodix_chicago_preprocessor_new(ctx->calibration, background, &error);
  if (!ctx->preprocessor)
    goto fail;
  ctx->gallery =
      gallery_len
          ? goodix_chicago_enrollment_unpack(gallery, gallery_len, &error)
          : goodix_chicago_enrollment_new();
  if (!ctx->gallery)
    goto fail;
  goodix_chicago_engine_enrollment_policy_init(&ctx->policy);
  return ctx;
fail:
  error_text(error, error_out, error_capacity);
  gxfp_chicago_free(ctx);
  return NULL;
}

static int insert(GxfpChicago *ctx, const GoodixChicagoSubtemplateView *v,
                  GoodixChicagoEnrollmentResult *result, GError **error) {
  if (goodix_chicago_enrollment_get_count(ctx->gallery) == 0)
    return goodix_chicago_enrollment_insert_first(
        ctx->gallery, v->records, v->record_count, v->active_count, v->quality,
        v->coverage, v->metric_data, result, error);
  return goodix_chicago_enrollment_insert_next(
      ctx->gallery, v->records, v->record_count, v->active_count, v->quality,
      v->coverage, v->metric_data, result, error);
}

int gxfp_chicago_process(GxfpChicago *ctx, const uint16_t *raw, int enroll,
                         GxfpResult *out, char *error_out,
                         size_t error_capacity) {
  GError *error = NULL;
  GoodixChicagoRuntimeReject reject = GOODIX_CHICAGO_RUNTIME_REJECT_NONE;
  g_autoptr(GoodixChicagoRuntimeProbe) probe = NULL;
  memset(out, 0, sizeof(*out));
  probe = goodix_chicago_runtime_prepare_probe(ctx->preprocessor, raw, &reject,
                                               &error);
  out->reject = reject;
  if (!probe) {
    if (error)
      goto fail;
    return 1;
  }
  const GoodixChicagoSubtemplateView *v =
      goodix_chicago_runtime_probe_get_view(probe);
  out->features = v->record_count;
  out->quality = v->quality;
  out->coverage = v->coverage;
  if (enroll) {
    if (goodix_chicago_enrollment_get_count(ctx->gallery) >=
        GOODIX_CHICAGO_ENROLLMENT_CAPACITY) {
      g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                          "Enrollment capacity reached");
      goto fail;
    }
    GoodixChicagoEnrollmentResult result;
    if (!insert(ctx, v, &result, &error))
      goto fail;
    out->position_x = result.position_x;
    out->position_y = result.position_y;
    out->accepted = goodix_chicago_engine_enrollment_policy_accept(
        &ctx->policy, result.position_x, result.position_y,
        &out->position_reject);
    if (ctx->policy.defer_current_sample) {
      if (ctx->deferred ||
          !goodix_chicago_enrollment_drop_last(ctx->gallery, &error))
        goto fail;
      ctx->deferred = g_steal_pointer(&probe);
    }
    if (ctx->policy.restore_deferred_sample) {
      if (!ctx->deferred)
        goto fail;
      const GoodixChicagoSubtemplateView *deferred =
          goodix_chicago_runtime_probe_get_view(ctx->deferred);
      if (!insert(ctx, deferred, &result, &error))
        goto fail;
      g_clear_pointer(&ctx->deferred, goodix_chicago_runtime_probe_free);
    }
  } else {
    GoodixChicagoMatchTemplateResult result;
    goodix_chicago_match_template_type24(
        ctx->gallery, v, GOODIX_CHICAGO_RUNTIME_SELECTOR_THRESHOLD, &result);
    out->score = result.score;
    out->accepted = result.score > 0;
  }
  out->count = ctx->policy.used;
  return 1;
fail:
  error_text(error, error_out, error_capacity);
  return 0;
}

/* Returns copied byte count, zero on error. Rust supplies bounded storage. */
size_t gxfp_chicago_export(GxfpChicago *ctx, int calibration, uint8_t *out,
                           size_t capacity, char *error_out,
                           size_t error_capacity) {
  GError *error = NULL;
  GBytes *bytes = calibration
                      ? g_bytes_ref(ctx->calibration)
                      : goodix_chicago_enrollment_pack(ctx->gallery, &error);
  if (!bytes) {
    error_text(error, error_out, error_capacity);
    return 0;
  }
  size_t length;
  const void *data = g_bytes_get_data(bytes, &length);
  if (length > capacity) {
    g_bytes_unref(bytes);
    return 0;
  }
  memcpy(out, data, length);
  g_bytes_unref(bytes);
  return length;
}
