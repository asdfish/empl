/*
 * empl - Extensible Music PLayer
 * Copyright (C) 2025  Andrew Chi
 *
 * This file is part of empl.
 *
 * empl is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * empl is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with empl.  If not, see <http://www.gnu.org/licenses/>.
 */

#ifndef REEXPORTS_H
#define REEXPORTS_H

#include <libguile.h>

extern _Bool reexports_scm_equal_p(SCM, SCM);

extern const SCM REEXPORTS_SCM_BOOL_F;
extern const SCM REEXPORTS_SCM_BOOL_T;
extern const SCM REEXPORTS_SCM_UNDEFINED;

#endif // REEXPORTS_H
