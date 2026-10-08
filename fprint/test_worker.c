// SPDX-License-Identifier: LGPL-3.0-or-later
// Exercise the actual adapter pipe reader with synthetic subprocess output.
#include "gxfp51b7.c"

static void check_stream(const gchar *script, gboolean enrollment,
                         gboolean cancelled, gint expected_error) {
  FpiDeviceGxfp51b7 *self = g_object_new(fpi_device_gxfp51b7_get_type(), NULL);
  GSubprocessLauncher *launcher = g_subprocess_launcher_new(
      G_SUBPROCESS_FLAGS_STDIN_PIPE | G_SUBPROCESS_FLAGS_STDOUT_PIPE);
  g_subprocess_launcher_set_child_setup(launcher, child_setup, NULL, NULL);
  GError *error = NULL;
  self->worker = g_subprocess_launcher_spawn(launcher, &error, "/bin/sh", "-c",
                                             script, NULL);
  g_assert_no_error(error);
  g_object_unref(launcher);
  GCancellable *cancel = g_cancellable_new();
  if (cancelled)
    g_cancellable_cancel(cancel);
  GTask *task = g_task_new(self, cancel, NULL, NULL);
  Request *request = g_new0(Request, 1);
  request->enroll = enrollment;
  g_task_set_task_data(task, request, (GDestroyNotify)request_free);
  task_run(task, self, request, cancel);
  Result *result = g_task_propagate_pointer(task, &error);
  if (expected_error >= 0) {
    g_assert_error(error, G_IO_ERROR, expected_error);
    g_assert_null(result);
  } else {
    g_assert_no_error(error);
    g_assert_nonnull(result);
    if (enrollment) {
      g_assert_nonnull(result->print);
      gsize length;
      const gchar *bytes = g_bytes_get_data(result->print, &length);
      g_assert_cmpmem(bytes, length, "{}", 2);
    } else
      g_assert_true(result->match);
  }
  g_clear_error(&error);
  result_free(result);
  g_clear_object(&self->worker);
  g_object_unref(task);
  g_object_unref(cancel);
  while (g_main_context_iteration(NULL, FALSE)) {
  }
  g_object_unref(self);
}
static void invalid_streams(void) {
  check_stream("printf 'garbage\\n'", FALSE, FALSE, G_IO_ERROR_INVALID_DATA);
  check_stream("printf 'progress 2\\n'", TRUE, FALSE, G_IO_ERROR_INVALID_DATA);
  check_stream("printf 'retry 5\\n'", FALSE, FALSE, G_IO_ERROR_INVALID_DATA);
  check_stream("printf 'waiting\\000garbage\\n'", FALSE, FALSE,
               G_IO_ERROR_INVALID_DATA);
  check_stream("printf 'print 2097153\\n'", TRUE, FALSE,
               G_IO_ERROR_INVALID_DATA);
  check_stream("printf 'print 2\\n{}'", TRUE, FALSE, G_IO_ERROR_INVALID_DATA);
  check_stream("printf 'match 1\\n'", TRUE, FALSE, G_IO_ERROR_INVALID_DATA);
  check_stream("printf 'waiting'", FALSE, FALSE, G_IO_ERROR_PARTIAL_INPUT);
  check_stream("printf '\\n'", FALSE, FALSE, G_IO_ERROR_INVALID_DATA);
}
static void result_and_cancellation(void) {
  check_stream("printf 'waiting\\ncaptured\\nmatch 1\\n'", FALSE, FALSE, -1);
  check_stream("sleep 30", FALSE, TRUE, G_IO_ERROR_CANCELLED);
}
static void enrollment_frames(void) {
  const gchar *stages = "i=1; while [ $i -le 12 ]; do printf 'progress %s\\n' "
                        "$i; i=$((i+1)); done; ";
  const gchar *frames[] = {"printf 'print 2\\n{}'", "printf 'print 3\\n{}'",
                           "printf 'print 2097153\\n'"};
  const gint errors[] = {-1, G_IO_ERROR_PARTIAL_INPUT, G_IO_ERROR_INVALID_DATA};
  for (guint i = 0; i < G_N_ELEMENTS(frames); i++) {
    g_autofree gchar *script = g_strconcat(stages, frames[i], NULL);
    check_stream(script, TRUE, FALSE, errors[i]);
  }
}
int main(int argc, char **argv) {
  g_test_init(&argc, &argv, NULL);
  g_test_add_func("/gxfp/worker/invalid-streams", invalid_streams);
  g_test_add_func("/gxfp/worker/result-and-cancellation",
                  result_and_cancellation);
  g_test_add_func("/gxfp/worker/enrollment-frames", enrollment_frames);
  return g_test_run();
}
