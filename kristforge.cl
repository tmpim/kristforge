#if defined(VECSIZE) && (VECSIZE == 1 || VECSIZE == 2 || VECSIZE == 4 || VECSIZE == 8 || VECSIZE == 16)

#else
#error Illegal vector size
#endif