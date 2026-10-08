// SPDX-License-Identifier: LGPL-3.0-or-later
#include <fcntl.h>
#include <fprint.h>
#include <gio/gio.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
static void progress(FpDevice *dev, gint stages, FpPrint *print, gpointer data,
                     GError *error) {
  (void)dev;
  (void)print;
  (void)data;
  fprintf(stderr, "PROGRESS=%d %s\n", stages,
          error ? error->message : "Lift, then place the finger again");
  fflush(stderr);
}
static gboolean matched = FALSE;
static void match(FpDevice *dev, FpPrint *print, FpPrint *scan, gpointer data,
                  GError *error) {
  (void)dev;
  (void)scan;
  (void)data;
  matched = print != NULL;
  fprintf(stderr, "MATCH=%d %s\n", matched, error ? error->message : "");
}
int main(int argc, char **argv) {
  if (argc < 2 || geteuid() != 0)
    return 2;
  g_autoptr(FpContext) context = fp_context_new();
  fp_context_enumerate(context);
  GPtrArray *devices = fp_context_get_devices(context);
  FpDevice *dev = NULL;
  for (guint i = 0; i < devices->len; i++) {
    FpDevice *d = g_ptr_array_index(devices, i);
    if (g_str_equal(fp_device_get_driver(d), "gxfp51b7"))
      dev = d;
  }
  if (!dev) {
    fprintf(stderr, "GXFP51B7 driver unavailable\n");
    return 2;
  }
  if (g_str_equal(argv[1], "list")) {
    printf("%s\n", fp_device_get_name(dev));
    return 0;
  }
  if (argc != 4)
    return 2;
  GError *error = NULL;
  g_autoptr(FpPrint) print = NULL;
  if (!fp_device_open_sync(dev, NULL, &error))
    goto fail;
  if (g_str_equal(argv[1], "enroll")) {
    print = fp_print_new(dev);
    fp_print_set_username(print, argv[2]);
    fp_print_set_finger(print, FP_FINGER_RIGHT_INDEX);
    FpPrint *complete =
        fp_device_enroll_sync(dev, print, NULL, progress, NULL, &error);
    if (!complete)
      goto fail;
    g_object_unref(print);
    print = complete;
    guchar *bytes = NULL;
    gsize size = 0;
    if (!fp_print_serialize(print, &bytes, &size, &error))
      goto fail;
    int fd = open(argv[3], O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0600);
    if (fd < 0) {
      g_free(bytes);
      fprintf(stderr, "Create private print failed\n");
      return 2;
    }
    gsize offset = 0;
    while (offset < size) {
      ssize_t n = write(fd, bytes + offset, size - offset);
      if (n <= 0) {
        close(fd);
        g_free(bytes);
        return 2;
      }
      offset += (gsize)n;
    }
    fsync(fd);
    close(fd);
    g_free(bytes);
    printf("ENROLL_RESULT=0\n");
  } else if (g_str_equal(argv[1], "verify")) {
    gchar *bytes = NULL;
    gsize size = 0;
    if (!g_file_get_contents(argv[3], &bytes, &size, &error))
      goto fail;
    print = fp_print_deserialize((guchar *)bytes, size, &error);
    g_free(bytes);
    if (!print)
      goto fail;
    gboolean verified = FALSE;
    if (!fp_device_verify_sync(dev, print, NULL, match, NULL, &verified, NULL,
                               &error))
      goto fail;
    printf("VERIFY_RESULT=%d\n", verified);
    fp_device_close_sync(dev, NULL, NULL);
    return verified ? 0 : 1;
  } else
    return 2;
  fp_device_close_sync(dev, NULL, NULL);
  return 0;
fail:
  fprintf(stderr, "%s\n",
          error ? error->message : "Fingerprint operation failed");
  g_clear_error(&error);
  return 2;
}
