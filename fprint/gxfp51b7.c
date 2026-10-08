// SPDX-License-Identifier: LGPL-3.0-or-later
// libfprint TOD adapter. Rust owns acquisition, enrollment and the matcher.
#include <drivers_api.h>
#include <gio/gio.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define HELPER "/usr/local/lib/gxfp51b7/gxfp51b7"
#define MAX_PRINT (2u * 1024u * 1024u)
typedef struct {
  FpDevice parent;
  GSubprocess *worker;
  guint serial, completed, timer;
} FpiDeviceGxfp51b7;
typedef FpDeviceClass FpiDeviceGxfp51b7Class;
G_DEFINE_TYPE(FpiDeviceGxfp51b7, fpi_device_gxfp51b7, FP_TYPE_DEVICE)

typedef struct {
  gboolean enroll;
  GBytes *input;
} Request;
typedef struct {
  GBytes *print;
  gboolean match;
} Result;
typedef struct {
  FpiDeviceGxfp51b7 *self;
  guint serial;
  gchar *line;
} Event;

static void result_free(Result *r) {
  if (r) {
    g_clear_pointer(&r->print, g_bytes_unref);
    g_free(r);
  }
}
static void request_free(Request *r) {
  g_clear_pointer(&r->input, g_bytes_unref);
  g_free(r);
}
static gboolean event_on_main(gpointer data) {
  Event *e = data;
  FpDevice *dev = FP_DEVICE(e->self);
  FpiDeviceAction action = fpi_device_get_current_action(dev);
  if (e->serial == e->self->serial && (action == FPI_DEVICE_ACTION_ENROLL ||
                                       action == FPI_DEVICE_ACTION_VERIFY)) {
    guint number = 0;
    if (g_str_equal(e->line, "waiting"))
      fpi_device_report_finger_status(dev, FP_FINGER_STATUS_NEEDED);
    else if (g_str_equal(e->line, "captured"))
      fpi_device_report_finger_status(dev, FP_FINGER_STATUS_PRESENT);
    else if (g_str_equal(e->line, "lift")) {
      fpi_device_report_finger_status(dev, FP_FINGER_STATUS_PRESENT);
      if (action == FPI_DEVICE_ACTION_ENROLL)
        fpi_device_enroll_progress(
            dev, (gint)e->self->completed, NULL,
            fpi_device_retry_new_msg(FP_DEVICE_RETRY_REMOVE_FINGER,
                                     "Lift the finger completely"));
    } else if (sscanf(e->line, "progress %u", &number) == 1 &&
               action == FPI_DEVICE_ACTION_ENROLL) {
      e->self->completed = number;
      fpi_device_enroll_progress(dev, (gint)number, NULL, NULL);
    } else if (sscanf(e->line, "retry %u", &number) == 1) {
      const gchar *message = number == 1   ? "Move the finger lower"
                             : number == 2 ? "Move the finger higher"
                             : number == 3 ? "Move the finger right"
                             : number == 4
                                 ? "Move the finger left"
                                 : "Cover the sensor fully and try again";
      GError *error = fpi_device_retry_new_msg(
          number ? FP_DEVICE_RETRY_CENTER_FINGER : FP_DEVICE_RETRY_GENERAL,
          "%s", message);
      if (action == FPI_DEVICE_ACTION_ENROLL)
        fpi_device_enroll_progress(dev, (gint)e->self->completed, NULL, error);
      else
        fpi_device_verify_report(dev, FPI_MATCH_ERROR, NULL, error);
    }
  }
  g_object_unref(e->self);
  g_free(e->line);
  g_free(e);
  return G_SOURCE_REMOVE;
}
static void report(FpiDeviceGxfp51b7 *self, const gchar *line) {
  Event *e = g_new0(Event, 1);
  e->self = g_object_ref(self);
  e->serial = self->serial;
  e->line = g_strdup(line);
  g_main_context_invoke(NULL, event_on_main, e);
}
static void child_setup(gpointer unused) {
  (void)unused;
  setpgid(0, 0);
}
static void stop_worker(FpiDeviceGxfp51b7 *self) {
  if (!self->worker)
    return;
  const gchar *id = g_subprocess_get_identifier(self->worker);
  if (id) {
    long pid = strtol(id, NULL, 10);
    if (pid > 1)
      kill(-(pid_t)pid, SIGKILL);
  }
  g_subprocess_force_exit(self->worker);
}
static gboolean deadline(gpointer data) {
  FpiDeviceGxfp51b7 *self = data;
  self->timer = 0;
  stop_worker(self);
  return G_SOURCE_REMOVE;
}
static gboolean number_event(const gchar *line, const gchar *prefix,
                             guint limit, guint *number) {
  if (!g_str_has_prefix(line, prefix) || !g_ascii_isdigit(line[strlen(prefix)]))
    return FALSE;
  gchar *end = NULL;
  guint64 value = g_ascii_strtoull(line + strlen(prefix), &end, 10);
  if (*end || value > limit)
    return FALSE;
  *number = (guint)value;
  return TRUE;
}
static void task_run(GTask *task, gpointer object, gpointer data,
                     GCancellable *cancel) {
  FpiDeviceGxfp51b7 *self = object;
  Request *request = data;
  GError *error = NULL;
  Result *result = g_new0(Result, 1);
  guint completed = 0;
  GInputStream *input = g_subprocess_get_stdout_pipe(self->worker);
  GOutputStream *output = g_subprocess_get_stdin_pipe(self->worker);
  if (request->input) {
    gsize length = 0;
    const guint8 *bytes = g_bytes_get_data(request->input, &length);
    guint32 header = GUINT32_TO_LE((guint32)length);
    if (!g_output_stream_write_all(output, &header, 4, NULL, cancel, &error) ||
        !g_output_stream_write_all(output, bytes, length, NULL, cancel, &error))
      goto fail;
  }
  if (!g_output_stream_close(output, cancel, &error))
    goto fail;
  for (;;) {
    gchar line[128];
    gsize count = 0;
    do {
      guint8 byte = 0;
      gssize n = g_input_stream_read(input, &byte, 1, cancel, &error);
      if (n <= 0) {
        if (!error)
          g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_PARTIAL_INPUT,
                              "Fingerprint worker ended before its result");
        goto fail;
      }
      if (byte == '\n')
        break;
      if (count >= sizeof(line) - 1) {
        g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                            "Oversized worker event");
        goto fail;
      }
      line[count++] = (gchar)byte;
    } while (TRUE);
    line[count] = 0;
    if (g_str_has_prefix(line, "print ")) {
      gchar *end = NULL;
      guint64 length = g_ascii_strtoull(line + 6, &end, 10);
      if (!request->enroll || completed != 12 || !g_ascii_isdigit(line[6]) ||
          *end || length == 0 || length > MAX_PRINT) {
        g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                            "Invalid worker print");
        goto fail;
      }
      guint8 *bytes = g_malloc((gsize)length);
      gsize actual = 0;
      if (!g_input_stream_read_all(input, bytes, (gsize)length, &actual, cancel,
                                   &error) ||
          actual != length) {
        g_free(bytes);
        if (!error)
          g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_PARTIAL_INPUT,
                              "Incomplete worker print");
        goto fail;
      }
      result->print = g_bytes_new_take(bytes, (gsize)length);
      break;
    }
    if (g_str_equal(line, "match 0") || g_str_equal(line, "match 1")) {
      if (request->enroll) {
        g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                            "Unexpected verification result");
        goto fail;
      }
      result->match = g_str_equal(line, "match 1");
      break;
    }
    guint number;
    if (number_event(line, "progress ", 12, &number)) {
      if (!request->enroll || number != completed + 1)
        goto invalid_event;
      completed = number;
    } else if (!(g_str_equal(line, "waiting") ||
                 g_str_equal(line, "captured") || g_str_equal(line, "lift") ||
                 number_event(line, "retry ", 4, &number))) {
    invalid_event:
      g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                          "Invalid worker event");
      goto fail;
    }
    report(self, line);
  }
  if (!g_subprocess_wait_check(self->worker, cancel, &error))
    goto fail;
  g_task_return_pointer(task, result, (GDestroyNotify)result_free);
  return;
fail:
  stop_worker(self);
  g_subprocess_wait(self->worker, NULL, NULL);
  result_free(result);
  g_task_return_error(task,
                      error ? error
                            : g_error_new_literal(G_IO_ERROR, G_IO_ERROR_FAILED,
                                                  "Fingerprint worker failed"));
}
static void finished(GObject *object, GAsyncResult *async, gpointer unused) {
  (void)unused;
  FpiDeviceGxfp51b7 *self = (FpiDeviceGxfp51b7 *)object;
  FpDevice *dev = FP_DEVICE(self);
  Request *request = g_task_get_task_data(G_TASK(async));
  GError *error = NULL;
  Result *result = g_task_propagate_pointer(G_TASK(async), &error);
  if (self->timer) {
    g_source_remove(self->timer);
    self->timer = 0;
  }
  self->serial++;
  g_clear_object(&self->worker);
  fpi_device_report_finger_status(dev, FP_FINGER_STATUS_NONE);
  if (error) {
    fpi_device_action_error(dev, error);
    return;
  }
  if (request->enroll) {
    FpPrint *print = NULL;
    fpi_device_get_enroll_data(dev, &print);
    gsize length;
    const guint8 *bytes = g_bytes_get_data(result->print, &length);
    GVariant *data =
        g_variant_new_fixed_array(G_VARIANT_TYPE_BYTE, bytes, length, 1);
    fpi_print_set_type(print, FPI_PRINT_RAW);
    fpi_print_set_device_stored(print, FALSE);
    g_object_set(print, "fpi-data", data, NULL);
    fpi_device_enroll_complete(dev, g_object_ref(print), NULL);
  } else {
    fpi_device_verify_report(
        dev, result->match ? FPI_MATCH_SUCCESS : FPI_MATCH_FAIL, NULL, NULL);
    fpi_device_verify_complete(dev, NULL);
  }
  result_free(result);
}
static void start(FpDevice *dev, gboolean enroll) {
  FpiDeviceGxfp51b7 *self = (FpiDeviceGxfp51b7 *)dev;
  FpPrint *print = NULL;
  GError *error = NULL;
  if (enroll)
    fpi_device_get_enroll_data(dev, &print);
  else
    fpi_device_get_verify_data(dev, &print);
  const gchar *user = print ? fp_print_get_username(print) : NULL;
  if (!user || !*user) {
    fpi_device_action_error(
        dev, fpi_device_error_new_msg(FP_DEVICE_ERROR_GENERAL,
                                      "An enrolled account is required"));
    return;
  }
  Request *request = g_new0(Request, 1);
  request->enroll = enroll;
  if (!enroll) {
    GVariant *data = NULL;
    g_object_get(print, "fpi-data", &data, NULL);
    if (!data || !g_variant_is_of_type(data, G_VARIANT_TYPE("ay"))) {
      g_clear_pointer(&data, g_variant_unref);
      request_free(request);
      fpi_device_action_error(
          dev, fpi_device_error_new(FP_DEVICE_ERROR_DATA_INVALID));
      return;
    }
    gsize length;
    const guint8 *bytes = g_variant_get_fixed_array(data, &length, 1);
    if (length == 0 || length > MAX_PRINT) {
      g_variant_unref(data);
      request_free(request);
      fpi_device_action_error(
          dev, fpi_device_error_new(FP_DEVICE_ERROR_DATA_INVALID));
      return;
    }
    request->input = g_bytes_new(bytes, length);
    g_variant_unref(data);
  }
  GSubprocessLauncher *launcher = g_subprocess_launcher_new(
      G_SUBPROCESS_FLAGS_STDIN_PIPE | G_SUBPROCESS_FLAGS_STDOUT_PIPE |
      G_SUBPROCESS_FLAGS_STDERR_SILENCE);
  gchar *env[] = {"PATH=/usr/bin:/bin", "LANG=C.UTF-8", NULL};
  g_subprocess_launcher_set_environ(launcher, env);
  g_subprocess_launcher_set_child_setup(launcher, child_setup, NULL, NULL);
  const gchar *args[] = {HELPER, "fprint-worker", user,
                         enroll ? "--enroll" : NULL, NULL};
  self->worker = g_subprocess_launcher_spawnv(launcher, args, &error);
  g_object_unref(launcher);
  if (!self->worker) {
    request_free(request);
    fpi_device_action_error(dev, error);
    return;
  }
  self->serial++;
  self->completed = 0;
  self->timer = g_timeout_add_seconds(enroll ? 660 : 30, deadline, self);
  GTask *task =
      g_task_new(self, fpi_device_get_cancellable(dev), finished, NULL);
  g_task_set_task_data(task, request, (GDestroyNotify)request_free);
  g_task_set_return_on_cancel(task, FALSE);
  g_task_run_in_thread(task, task_run);
  g_object_unref(task);
}
static void enroll(FpDevice *dev) { start(dev, TRUE); }
static void verify(FpDevice *dev) { start(dev, FALSE); }
static void cancel(FpDevice *dev) { stop_worker((FpiDeviceGxfp51b7 *)dev); }
static void probe(FpDevice *dev) {
  if (geteuid() != 0 ||
      g_strcmp0(fpi_device_get_virtual_env(dev), "/dev/goodix_bios_sealed") !=
          0 ||
      !g_file_test("/sys/bus/acpi/devices/GXFP51B7:00", G_FILE_TEST_IS_DIR)) {
    fpi_device_probe_complete(
        dev, NULL, NULL, fpi_device_error_new(FP_DEVICE_ERROR_NOT_SUPPORTED));
    return;
  }
  fpi_device_probe_complete(dev, "gxfp51b7-ec-chicagohs",
                            "GXFP51B7 fingerprint sensor", NULL);
}
static void open_device(FpDevice *dev) { fpi_device_open_complete(dev, NULL); }
static void close_device(FpDevice *dev) {
  fpi_device_close_complete(dev, NULL);
}
static void fpi_device_gxfp51b7_init(FpiDeviceGxfp51b7 *self) { (void)self; }
static void fpi_device_gxfp51b7_class_init(FpiDeviceGxfp51b7Class *klass) {
  static const FpIdEntry ids[] = {{.virtual_envvar = "FP_GXFP51B7_DEVICE"},
                                  {.virtual_envvar = NULL}};
  klass->id = "gxfp51b7";
  klass->full_name = "GXFP51B7 EC ChicagoHS";
  klass->type = FP_DEVICE_TYPE_VIRTUAL;
  klass->id_table = ids;
  klass->nr_enroll_stages = 12;
  klass->scan_type = FP_SCAN_TYPE_PRESS;
  klass->temp_hot_seconds = -1;
  klass->probe = probe;
  klass->open = open_device;
  klass->close = close_device;
  klass->enroll = enroll;
  klass->verify = verify;
  klass->cancel = cancel;
  fpi_device_class_auto_initialize_features(klass);
}
GType fpi_tod_shared_driver_get_type(void) {
  return fpi_device_gxfp51b7_get_type();
}
