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

#include <libguile.h>

#include "reexports.h"

_Bool reexports_scm_equal_p(SCM x, SCM y) {
  return scm_equal_p(x, y);
}

const SCM REEXPORTS_SCM_BOOL_F = SCM_BOOL_F;
const SCM REEXPORTS_SCM_BOOL_T = SCM_BOOL_T;
const SCM REEXPORTS_SCM_UNDEFINED = SCM_UNDEFINED;
